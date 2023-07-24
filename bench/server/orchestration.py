import asyncio
import time
from datetime import datetime
from typing import Collection
from uuid import UUID

import structlog

from bench import models
from bench.language.session import WorkerSetStatus
from bench.models import packer
from bench.models.worker import WORKER_SET_FIELDS_NO_ID
from bench.msg import nc_init
from bench.msg.core import NMessage, handle_reply, message_handler, publish
from bench.msg.messages import (
    NMessageType,
    RepConfigureWorkerSetPayload,
    RepWakeWorkerSetPayload,
    ReqConfigureWorkerSetPayload,
    ReqRestartWorkerSetPayload,
    ReqWakeWorkerSetPayload,
    WorkersChangedPayload,
)
from bench.server import k8
from bench.server.k8 import K8_AVAILABLE
from bench.utils.cache import redis
from bench.utils.func import wrap_task
from bench.utils.monitoring import Monitored
from bench.utils.utils import DEBUG, sentry_capture_if_enabled
from bench.utils.uuidt import UUIDT

logger = structlog.get_logger(__name__)

USE_K8 = K8_AVAILABLE or not DEBUG
WORKER_SET_IDLE_SLEEP_TIME = 60 * 20  # 20 minutes


def create_wrapped_task(coro, task_id: str = None):
    asyncio.create_task(wrap_task(coro, task_id))


class OrchestrationServer(Monitored):
    """
    Singleton server to orchestrate workers in k8s requests.
    There can only be one master globally for now, which is "enforced" by deploying as StatefulSet.
    TODO @Robustness: use k8 lease for master server election (failover)
    """

    def __init__(self):
        self.id = UUIDT()
        self.subs = []
        self.tasks = []
        self.worker_sets_by_project_id: dict[UUID, models.WorkerSet] = {}
        self._ready = False

    @property
    def ready(self):
        return self._ready

    async def run(self):
        await k8.init()
        await nc_init.wait()
        logger.info("start")
        worker_sets = [
            ws
            async for ws in models.WorkerSet.objects.select_related(
                "project", "project__organization", "project__user"
            ).all()
        ]
        for worker_set in worker_sets:
            self.worker_sets_by_project_id[worker_set.project_id] = worker_set
        logger.debug("worker_sets.loaded", worker_sets=self.worker_sets_by_project_id.values())

        #
        self.subs = [
            await handle_reply(NMessageType.CONFIGURE_WORKER_SET, self.configure_worker_set),
            await handle_reply(NMessageType.WAKE_WORKER_SET, self.wake_worker_set),
            await handle_reply(NMessageType.RESTART_WORKER_SET, self.restart_worker_set),
        ]
        if USE_K8:
            self.tasks.append(asyncio.create_task(self._watch_worker_sets_in_k8_forever()))
        self.tasks.append(asyncio.create_task(self._manage_worker_lifecycle_forever()))
        self._ready = True

    async def stop(self):
        logger.info("stop")
        self._ready = False
        for sub in self.subs:
            sub.unsubscribe()
        for task in self.tasks:
            task.cancel()
        await asyncio.gather(*self.tasks, return_exceptions=True)

    async def _write_worker_sets(self, worker_sets: Collection[models.WorkerSet]) -> None:
        """Write worker sets to DB, update in K8 and notify changes."""
        if not worker_sets:
            return
        if K8_AVAILABLE:  # update k8 deployments
            deployments = [k8.Deployment.from_model(worker_set) for worker_set in worker_sets]
            await k8.update_deployments(deployments)
        else:  # pretend they're all as needed
            for worker_set in worker_sets:
                worker_set.available_replicas = worker_set.target_replicas
                worker_set.ready_replicas = worker_set.target_replicas
                worker_set.status = (
                    WorkerSetStatus.SLEEPING if worker_set.sleeping else WorkerSetStatus.HEALTHY
                )
        # save in DB
        await models.WorkerSet.objects.abulk_update(worker_sets, WORKER_SET_FIELDS_NO_ID)
        # notify
        await publish(
            NMessageType.WORKERS_CHANGED,
            WorkersChangedPayload(
                project_id=worker_sets[0].project_id if len(worker_sets) == 1 else None,
                worker_sets=[packer.pack_data(worker_set) for worker_set in worker_sets],
            ),
        )

    async def _watch_worker_sets_in_k8_forever(self):
        """Watch k8 deployments and update worker sets accordingly."""
        async for event_type, deployment in k8.watch_our_deployments():
            partial_worker_set = deployment.to_model()
            worker_set = self.worker_sets_by_project_id.get(partial_worker_set.project_id)
            if worker_set is None:
                logger.warning("worker_set.unknown", worker_set=partial_worker_set)
                continue  # delete maybe?
            # update worker set
            for field in k8.DEPLOYMENT_DYNAMIC_FIELDS:
                setattr(worker_set, field, getattr(partial_worker_set, field))
            logger.info("worker_set.update", worker_set=worker_set)
            await worker_set.asave(force_update=True, update_fields=k8.DEPLOYMENT_DYNAMIC_FIELDS)
            # notify
            await publish(
                NMessageType.WORKERS_CHANGED,
                WorkersChangedPayload(
                    project_id=worker_set.project_id, worker_sest=[packer.pack_data(worker_set)]
                ),
            )

    async def _manage_worker_lifecycle_forever(self, interval: int = 60):
        """Puts worker sets to sleep after inactivity if needed."""
        while True:
            await asyncio.sleep(interval)
            idle_cutoff = time.time() - WORKER_SET_IDLE_SLEEP_TIME
            # scan iter "worker_set.{id}" in redis
            active_keys = await redis.keys("worker_set.*.last_active_at")
            active_values = await redis.mget(active_keys)

            for key, last_active_at in zip(active_keys, active_values):
                # update worker set
                worker_set_id = UUID(key.split(".")[1])
                last_active_at = float(last_active_at)
                worker_set = self.worker_sets_by_project_id.get(worker_set_id)
                if worker_set is None or worker_set.status != WorkerSetStatus.HEALTHY:
                    await redis.delete(key)  # delete old keys
                    continue
                worker_set.last_active_at = datetime.fromtimestamp(last_active_at)

                # put to sleep if idle for too long
                if last_active_at < idle_cutoff:
                    logger.info("worker_sets.sleep", worker_set=worker_set)
                    worker_set.sleeping = True
                    worker_set.target_replicas = 0

            await self._write_worker_sets(self.worker_sets_by_project_id.values())

    async def _get_project_worker_set(self, project_id: UUID):
        """Get default worker set for a project."""
        worker_set = self.worker_sets_by_project_id.get(project_id)
        if worker_set is None:
            # newly created project, get from DB
            project = await models.Project.objects.select_related(
                "worker_set", "worker_set__project"
            ).aget(id=project_id)
            self.worker_sets_by_project_id[project_id] = project.worker_set
            return project.worker_set
        else:
            return worker_set

    @message_handler
    async def configure_worker_set(self, msg: NMessage[ReqConfigureWorkerSetPayload]) -> None:
        try:
            worker_set = await self._get_project_worker_set(msg.p.project_id)
            worker_set.desired_replicas = msg.p.desired_replicas
            worker_set.profile = msg.p.profile
            worker_set.region = msg.p.region
            await self._write_worker_sets([worker_set])
            success = True
            logger.info("worker_sets.configure", worker_set=worker_set)
        except Exception as e:
            sentry_capture_if_enabled(e)
            logger.error("worker_sets.configure.failed", msg=msg, exc_info=True)
            success = False
        await msg.reply(RepConfigureWorkerSetPayload(success=success))

    @message_handler
    async def wake_worker_set(self, msg: NMessage[ReqWakeWorkerSetPayload]) -> None:
        try:
            worker_set = await self._get_project_worker_set(msg.p.project_id)
            worker_set.sleeping = False
            worker_set.target_replicas = worker_set.desired_replicas
            worker_set.status = WorkerSetStatus.PENDING
            logger.info("worker_sets.wake", worker_sets=worker_set)
            await self._write_worker_sets([worker_set])
            success = True
        except Exception as e:
            sentry_capture_if_enabled(e)
            logger.error("worker_sets.wake.failed", msg=msg, exc_info=True)
            success = False
            worker_set = None
        await msg.reply(
            RepWakeWorkerSetPayload(
                worker_set_id=worker_set.id if worker_set else None, success=success
            )
        )

    @message_handler
    async def restart_worker_set(self, msg: NMessage[ReqRestartWorkerSetPayload]) -> None:
        success = False
        try:
            if not USE_K8:
                # touch 'manage.py' file to trigger reload in dev mode
                with open("manage.py", "a"):
                    success = True
            else:
                worker_set = await self._get_project_worker_set(msg.p.project_id)
                if worker_set.status == WorkerSetStatus.HEALTHY:
                    await k8.restart_deployment(k8.Deployment.from_model(worker_set))
                    success = True
                logger.info("worker_sets.restart", worker_set=worker_set)
        except Exception as e:
            sentry_capture_if_enabled(e)
            logger.error("worker_sets.restart.failed", msg=msg, exc_info=True)
        await msg.reply(RepConfigureWorkerSetPayload(success=success))
