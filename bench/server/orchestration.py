import asyncio
from datetime import datetime, timedelta
from typing import Collection, Optional
from uuid import UUID

import structlog
from asgiref.sync import sync_to_async
from django.db.models import Q

from bench import models
from bench.language.const import ACTIVE_RUN_STATUSES, PENDING_RUN_STATUSES, WorkerSetStatus
from bench.models import packer
from bench.models.worker import WORKER_SET_FIELDS_NO_ID
from bench.msg import nc_init
from bench.msg.core import NMessage, handle_reply, message_handler, publish, request
from bench.msg.messages import (
    NMessageType,
    RepConfigureWorkerSetPayload,
    RepDoRestartWorkerNodePayload,
    RepRestartWorkerSetPayload,
    RepWakeWorkerSetPayload,
    ReqConfigureWorkerSetPayload,
    ReqDoRestartWorkerNodePayload,
    ReqRestartWorkerSetPayload,
    ReqWakeWorkerSetPayload,
    RunsChangedGlobalPayload,
    WorkersChangedPayload,
)
from bench.opensearch.index import write_runs_to_os
from bench.server import k8
from bench.settings import KUBERNETES_ENABLED
from bench.utils.cache import redis
from bench.utils.dt import utcnow_with_tz
from bench.utils.monitoring import Monitored
from bench.utils.task import TaskManager
from bench.utils.utils import sentry_capture
from bench.utils.uuidt import UUIDT

logger = structlog.get_logger(__name__)

WORKER_SET_IDLE_SLEEP_TIME = 30 * 60  # 30 minutes
WORKER_SET_GENTLE_RESTART_TIMEOUT = 5  # 5 seconds until force restart


class OrchestrationServer(Monitored):
    """
    Singleton server to orchestrate workers in k8s requests.
    There can only be one master globally for now, which is "enforced" by deploying as StatefulSet.
    TODO @Robustness: use k8 lease for master server election (failover)
    """

    def __init__(self):
        self.id = UUIDT()
        self.subs = []
        self.tasks = TaskManager()
        self.worker_sets_by_project_id: dict[UUID, models.WorkerSet] = {}
        self.worker_sets_by_id: dict[UUID, models.WorkerSet] = {}
        self._ready = False

    @property
    def ready(self):
        return self._ready

    @property
    def healthy(self):
        return self.tasks.healthy and self.ready

    @property
    def worker_sets(self) -> Collection[models.WorkerSet]:
        return self.worker_sets_by_project_id.values()

    async def run(self):
        await k8.init()
        await nc_init.wait()
        logger.info("start")

        # load
        worker_sets = [
            ws
            async for ws in models.WorkerSet.objects.select_related(
                "project", "project__organization", "project__user"
            ).all()
        ]
        for worker_set in worker_sets:
            self.worker_sets_by_project_id[worker_set.project_id] = worker_set
            self.worker_sets_by_id[worker_set.id] = worker_set
        logger.debug("workers.loaded_from_db", worker_sets=self.worker_sets)

        # initial sync
        if KUBERNETES_ENABLED:
            # fetch from k8
            k8_deployments, k8_revision_mark = await k8.get_all_deployments()
            k8_deployments_to_kill: list[k8.Deployment] = []
            active_worker_node_ids: list[str] = []
            dead_worker_node_ids: set[str] = set()
            for k8_deployment in k8_deployments:
                worker_set = self.worker_sets_by_project_id.get(k8_deployment.project_id)
                if not worker_set:  # shouldn't exist anymore
                    k8_deployments_to_kill.append(k8_deployment)
                    continue
                for node_id in worker_set.active_replicas_ids:
                    if node_id not in k8_deployment.active_replicas_ids:
                        dead_worker_node_ids.add(node_id)
                worker_set.status = k8_deployment.status
                worker_set.ready_replicas = k8_deployment.ready_replicas
                worker_set.active_replicas_ids = k8_deployment.active_replicas_ids
                active_worker_node_ids.extend(k8_deployment.active_replicas_ids)

            if k8_deployments_to_kill:
                await k8.delete_deployments(k8_deployments_to_kill)

            # re-deploy as needed, start listening
            await self._update_last_active_from_redis()
            await self._mark_tired_worker_sets()
            await self._save_and_notify_worker_sets(self.worker_sets)
            self.tasks.start(self._watch_worker_sets_in_k8_forever(k8_revision_mark))
            await self._deploy_worker_sets(self.worker_sets)
        else:  # mark local as deadish (just started)
            dead_worker_node_ids = {"local"}
            active_worker_node_ids = []

        # also extend dead nodes by any worker nodes that have an active run but aren't in k8
        # (this can happen to any number of glitches in our run/worker tracking)
        presumed_dead_worker_node_ids: set[str] = {
            i
            async for i in models.Run.objects.filter(
                worker_node_id__isnull=False, status__in=PENDING_RUN_STATUSES
            ).values_list("worker_node_id", flat=True)
            if i not in active_worker_node_ids
        }
        dead_worker_node_ids = set(dead_worker_node_ids) | presumed_dead_worker_node_ids
        if dead_worker_node_ids:
            await self._mark_runs_dead(project_id=None, worker_node_ids=dead_worker_node_ids)

        # start for real
        self.tasks.start(self._manage_worker_lifecycle_forever())
        self.subs = [
            await handle_reply(NMessageType.CONFIGURE_WORKER_SET, self.configure_worker_set),
            await handle_reply(NMessageType.WAKE_WORKER_SET, self.wake_worker_set),
            await handle_reply(NMessageType.RESTART_WORKER_SET, self.restart_worker_set),
        ]
        self._ready = True
        logger.info("ready")

    async def stop(self):
        logger.info("stop")
        self._ready = False
        for sub in self.subs:
            sub.unsubscribe()
        await self.tasks.stop()

    async def _deploy_worker_sets(self, worker_sets: Collection[models.WorkerSet]) -> None:
        """Deploy worker sets in K8 (if available)."""
        logger.debug("workers.deploy", worker_sets=worker_sets)
        if not worker_sets:
            return
        if KUBERNETES_ENABLED:  # update k8 deployments
            deployments = [k8.Deployment.from_model(worker_set) for worker_set in worker_sets]
            await k8.update_deployments(deployments)
        else:  # pretend they're all as needed
            for worker_set in worker_sets:
                worker_set.available_replicas = worker_set.target_replicas
                worker_set.ready_replicas = worker_set.target_replicas
                worker_set.status = (
                    WorkerSetStatus.SLEEPING if worker_set.sleeping else WorkerSetStatus.HEALTHY
                )
            await self._save_and_notify_worker_sets(worker_sets)

    async def _save_and_notify_worker_sets(self, worker_sets: Collection[models.WorkerSet]) -> None:
        logger.debug("workers.save_and_notify", worker_sets=worker_sets)
        # save in DB
        await models.WorkerSet.objects.abulk_update(worker_sets, WORKER_SET_FIELDS_NO_ID)
        # notify
        worker_sets_data = [packer.pack_data(worker_set) for worker_set in worker_sets]
        project_id = worker_sets_data[0].project_id if len(worker_sets) == 1 else None
        await publish(
            NMessageType.WORKERS_CHANGED,
            WorkersChangedPayload(project_id=project_id, worker_sets=worker_sets_data),
        )

    async def _mark_runs_dead(
        self,
        project_id: Optional[UUID],
        worker_node_ids: Collection[str],
        run_ids: Optional[Collection[UUID]] = None,
    ):
        """Marks runs on worker nodes as aborted (node may be lost or just restarting)."""
        logger.debug("mark_runs_dead", worker_node_ids=worker_node_ids, run_ids=run_ids)

        if project_id is not None:
            # extend worker_node_ids by any worker nodes that have an active run but aren't in k8
            worker_set = self.worker_sets_by_project_id.get(project_id)
            active_runs = models.Run.objects.filter(
                worker_node_id__isnull=False, status__in=ACTIVE_RUN_STATUSES
            ).values_list("worker_node_id", flat=True)
            also_dead_worker_node_ids = {
                i async for i in active_runs if i not in worker_set.active_replicas_ids
            }
            worker_node_ids = set(worker_node_ids) | also_dead_worker_node_ids

        # collect presumed dead runs
        dead_runs = models.Run.objects.filter(
            Q(worker_node_id__in=worker_node_ids, status__in=ACTIVE_RUN_STATUSES)
            | Q(id__in=run_ids or [])
        )
        if project_id:
            dead_runs = dead_runs.filter(project_id=project_id)
        dead_runs = [r async for r in dead_runs]

        # mark dead and send out updates
        if not dead_runs:
            return
        for run in dead_runs:
            run.mark_dead()
        await models.Run.objects.abulk_update(dead_runs, ["status", "terminated_at"])
        dead_runs_data = [packer.pack_data(r) for r in dead_runs]
        await sync_to_async(write_runs_to_os)(dead_runs_data)
        await publish(NMessageType.RUNS_CHANGED, RunsChangedGlobalPayload(runs=dead_runs_data))

    async def _watch_worker_sets_in_k8_forever(self, k8_marker: k8.VersionMarker):
        """Watch k8 deployments and update worker sets accordingly."""
        async for event_type, object in k8.watch_our_deployments(k8_marker):
            logger.debug("k8.event", event_type=event_type, object=object)
            worker_set = self.worker_sets_by_project_id.get(object.project_id)
            if worker_set is None:
                logger.warning("workers.unknown", object=object)
                continue  # delete maybe?

            if isinstance(object, k8.Deployment):
                # deployment / worker set changed
                partial_worker_set = object.to_model()
                worker_set.target_replicas = partial_worker_set.target_replicas
                worker_set.status = partial_worker_set.status
                if not worker_set.sleeping:
                    worker_set.last_bumped_at = utcnow_with_tz()
            elif isinstance(object, k8.Pod):
                # pod / worker node changed
                if event_type == k8.EventType.ADDED:
                    worker_set.active_replicas_ids.append(object.name)
                elif event_type == k8.EventType.DELETED:
                    if object.name not in worker_set.active_replicas_ids:
                        logger.warning("workers.unknown_pod", object=object)
                        continue
                    worker_set.active_replicas_ids.remove(object.name)
                    await self._mark_runs_dead(worker_set.project_id, [object.name])
                else:
                    continue
            else:
                raise TypeError(f"unexpected k8 object type: {type(object)}")
            logger.info("workers.update", worker_set=worker_set)
            await self._save_and_notify_worker_sets([worker_set])

    async def _manage_worker_lifecycle_forever(self, interval: int = 60):
        """Puts worker sets to sleep after inactivity if needed."""
        while True:
            await self._update_last_active_from_redis()
            tired_worker_sets = await self._mark_tired_worker_sets()
            logger.debug("workers.update_lifecycle", worker_sets=tired_worker_sets)
            await self._save_and_notify_worker_sets(self.worker_sets)
            if tired_worker_sets:
                await self._deploy_worker_sets(tired_worker_sets)
            await asyncio.sleep(interval)

    async def _update_last_active_from_redis(self):
        """
        Updates last_active_at state for each worker set from redis WITHOUT writing to DB.
        """
        # scan "workers.{set_id}.{node_id}.<val>" :WorkerSetActive
        logger.debug("workers.update_last_active_from_redis")
        active_keys = await redis.keys("worker_set.*.*.last_active_at")
        active_values = await redis.mget(active_keys)
        keys_to_delete = []
        for key, last_active_at in zip(active_keys, active_values):
            # update worker set
            try:
                worker_set_id = UUID(key.decode().split(".")[1])
                worker_set = self.worker_sets_by_id[worker_set_id]
                worker_set.last_active_at = datetime.fromisoformat(last_active_at.decode())
            except (ValueError, KeyError, IndexError, TypeError) as e:
                logger.warning("workers.last_active_at.invalid", key=key, error=e)
                keys_to_delete.append(key)  # delete invalid/stale keys
                continue
        if keys_to_delete:
            await redis.delete(*keys_to_delete)
        logger.debug(
            "workers.mark_last_active_from_redis.done",
            active_keys=len(active_keys),
            keys_to_delete=len(keys_to_delete),
        )

    async def _mark_tired_worker_sets(self) -> list[models.WorkerSet]:
        """Marks worker sets as sleeping if they are idle for too long WITHOUT writing to DB."""
        logger.debug("workers.mark_tired")
        tired_worker_sets = []
        idle_cutoff = utcnow_with_tz() - timedelta(seconds=WORKER_SET_IDLE_SLEEP_TIME)
        for worker_set in self.worker_sets:
            # put to sleep if idle for too long
            if (
                worker_set.last_active_at is None or worker_set.last_active_at < idle_cutoff
            ) and not (worker_set.last_bumped_at and worker_set.last_bumped_at > idle_cutoff):
                worker_set.sleeping = True
                worker_set.target_replicas = 0
                tired_worker_sets.append(worker_set)
        logger.debug("workers.mark_tired.done", worker_sets=tired_worker_sets)
        return tired_worker_sets

    async def _get_project_worker_set(self, project_id: UUID):
        """Get worker set for a project (load if not already loaded, may have just been created)."""
        worker_set = self.worker_sets_by_project_id.get(project_id)
        if worker_set is None:
            # newly created project, get from DB
            project = await models.Project.objects.select_related(
                "worker_set",
                "worker_set__project",
                "worker_set__project__user",
                "worker_set__project__organization",
            ).aget(id=project_id)
            self.worker_sets_by_project_id[project_id] = project.worker_set
            self.worker_sets_by_id[project.worker_set.id] = project.worker_set
            return project.worker_set
        else:
            return worker_set

    @message_handler
    async def configure_worker_set(self, msg: NMessage[ReqConfigureWorkerSetPayload]) -> None:
        try:
            logger.info("workers.configure", msg=msg)
            worker_set = await self._get_project_worker_set(msg.p.project_id)
            worker_set.desired_replicas = msg.p.desired_replicas
            worker_set.profile = msg.p.profile
            worker_set.region = msg.p.region
            worker_set.last_bumped_at = utcnow_with_tz()
            await self._save_and_notify_worker_sets([worker_set])
            await self._deploy_worker_sets([worker_set])
            success = True
            logger.info("workers.configure.done", msg=msg, worker_set=worker_set)
        except Exception as e:
            sentry_capture(e)
            logger.error("workers.configure.failed", msg=msg, exc_info=True)
            success = False
        await msg.reply(RepConfigureWorkerSetPayload(success=success))

    @message_handler
    async def wake_worker_set(self, msg: NMessage[ReqWakeWorkerSetPayload]) -> None:
        try:
            logger.info("workers.wake", msg=msg)
            worker_set = await self._get_project_worker_set(msg.p.project_id)
            was_sleeping = worker_set.sleeping
            if was_sleeping:
                worker_set.sleeping = False
                worker_set.target_replicas = worker_set.desired_replicas
                worker_set.status = WorkerSetStatus.PENDING
            worker_set.last_bumped_at = utcnow_with_tz()
            await self._save_and_notify_worker_sets([worker_set])
            if was_sleeping:
                await self._deploy_worker_sets([worker_set])
            logger.info("workers.wake.done", msg=msg, worker_sets=worker_set)
            success = True
        except Exception as e:
            sentry_capture(e)
            logger.error("workers.wake.failed", msg=msg, exc_info=True)
            success = False
            worker_set = None
        await msg.reply(
            RepWakeWorkerSetPayload(
                worker_set_id=worker_set.id if worker_set else None, success=success
            )
        )

    @message_handler
    async def restart_worker_set(self, msg: NMessage[ReqRestartWorkerSetPayload]) -> None:
        worker_set = await self._get_project_worker_set(msg.p.project_id)
        logger.info("workers.restart", worker_set=worker_set)
        success = False

        # restart (if we have any nodes)
        dead_replicas_ids = worker_set.active_replicas_ids[:]  # may change during restart
        if worker_set.target_replicas > 0:
            try:
                # TODO @Robustness: do restart worker node only works with 1 worker node
                rep: NMessage[RepDoRestartWorkerNodePayload] = await request(
                    NMessageType.DO_RESTART_WORKER_NODE,
                    ReqDoRestartWorkerNodePayload(
                        project_id=msg.p.project_id,
                        worker_set_id=worker_set.id,
                        worker_node_id=None,
                        worker_process_id=None,
                    ),
                    reply_t=RepDoRestartWorkerNodePayload,
                    timeout=WORKER_SET_GENTLE_RESTART_TIMEOUT,
                )
                success = rep.p.success
                logger.info("workers.restart.done", msg=msg, worker_set=worker_set)
            except Exception as e:
                sentry_capture(e)
                logger.error("workers.restart.failed", msg=msg, exc_info=True)
            if not success and KUBERNETES_ENABLED:
                await k8.restart_deployment(k8.Deployment.from_model(worker_set))
                success = True

        # mark all worker set nodes as deadish
        if KUBERNETES_ENABLED:
            await self._mark_runs_dead(msg.p.project_id, dead_replicas_ids)
        else:
            await self._mark_runs_dead(msg.p.project_id, ["local"])

        await msg.reply(
            RepRestartWorkerSetPayload(
                worker_set_id=worker_set.id if worker_set else None, success=success
            )
        )
