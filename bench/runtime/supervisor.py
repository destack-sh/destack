import asyncio
from datetime import datetime, timedelta
from typing import Collection, Optional
from uuid import UUID

import structlog
from asgiref.sync import sync_to_async
from django.db.models import Q

from bench.language.const import ACTIVE_RUN_STATUSES, PENDING_RUN_STATUSES, WorkerSetStatus
from bench.runtime import k8
from bench.runtime.host import RuntimeHost
from bench.search.engine import write_runs_to_os
from bench.settings import KUBERNETES_ENABLED
from bench.utils.cache import redis
from bench.utils.dt import utcnow_with_tz
from bench.utils.func import group_by
from bench.utils.monitoring import Monitored
from bench.utils.task import TaskManager
from bench.utils.utils import sentry_capture
from bench.utils.uuidt import UUIDT

logger = structlog.get_logger(__name__)

WORKER_SET_IDLE_SLEEP_TIME = 30 * 60  # 30 minutes
WORKER_SET_GENTLE_RESTART_TIMEOUT = 5  # 5 seconds until force restart


def _get_benches_to_manage() -> list[models.Bench]:
    benches = list(models.Bench.objects.all())  # obviously will shard this later

    # sanity check if default libs in code match those in DB
    for lib in DEFAULT_MODULES.values():
        bench = first((p for p in benches if p.id == lib.ck), None)
        if bench is None:
            raise RuntimeError(f"default lib {lib} not found in DB")
        if bench.head_id != lib.id:
            raise RuntimeError(f"default lib {lib} head mismatch with {bench}: {bench.head}")
    # and then exclude default benches since there's nothing to manage
    benches = [p for p in benches if p.path not in DEFAULT_MODULES]

    return benches


class RuntimeSupervisor(Monitored):
    def __init__(self):
        self.id = UUIDT()
        self.runtimes: dict[UUID, RuntimeHost] = {}
        self.tasks = TaskManager()
        self._ready = False
        self._worker_sets_by_bench_id: dict[UUID, models.WorkerSet] = {}
        self._worker_healthy_waiters: dict[UUID, asyncio.Event] = {}
        self._worker_sets_by_id: dict[UUID, models.WorkerSet] = {}

    @property
    def worker_sets(self) -> Collection[models.WorkerSet]:
        return self._worker_sets_by_bench_id.values()

    async def _on_workers_changed(self, msg: NMessage[WorkersChangedPayload]):
        logger.debug("worker_observer.change", msg=msg)
        for updated_ws in msg.p.worker_sets:
            # upsert properties in local worker set
            updated_ws: models.WorkerSet = packer.unpack_struct(updated_ws)
            if updated_ws.bench_id not in self._worker_sets_by_bench_id:
                ws = await models.WorkerSet.objects.select_related(
                    "bench", "bench__organization", "bench__user"
                ).aget(bench_id=updated_ws.bench_id)
            else:
                ws = self._worker_sets_by_bench_id[updated_ws.bench_id]
            for field in models.WorkerSet._meta.fields:
                # skip relational fields
                if field.is_relation:
                    continue
                setattr(ws, field.name, getattr(updated_ws, field.name))

            # fire 'until healthy' wait events
            if ws.status == WorkerSetStatus.HEALTHY and ws.bench_id in self._worker_healthy_waiters:
                self._worker_healthy_waiters[ws.bench_id].set()

    def is_worker_set_healthy(self, bench_id: UUID) -> bool:
        """Return whether the worker set is healthy."""
        worker_set = self._worker_sets_by_bench_id.get(bench_id)
        return worker_set and worker_set.status == WorkerSetStatus.HEALTHY

    def get_worker_set(self, bench_id: UUID) -> Optional[models.WorkerSet]:
        """Return the worker set if it exists."""
        return self._worker_sets_by_bench_id.get(bench_id)

    async def wake_until_healthy(self, bench_id: UUID, timeout: Optional[int] = None):
        """If not already healthy, wake the worker set and wait until it is healthy."""
        worker_set = self._worker_sets_by_bench_id.get(bench_id)
        log = logger.bind(bench_id=bench_id, worker_set=worker_set)
        log.info("worker_observer.wait_until_healthy")
        if worker_set and worker_set.status == WorkerSetStatus.HEALTHY:
            return  # already good

        # create waiter
        if bench_id not in self._worker_healthy_waiters:
            self._worker_healthy_waiters[bench_id] = asyncio.Event()

        # wake if needed
        if not worker_set or worker_set.sleeping:
            log.info("worker_observer.wait_until_healthy.wake")
            rep: NMessage[RepWakeWorkerSetPayload] = await request(
                NMessageType.WAKE_WORKER_SET,
                ReqWakeWorkerSetPayload(bench_id=bench_id),
                reply_t=RepWakeWorkerSetPayload,
                retry=3,
            )
            if not rep.p.success:
                raise RuntimeError(f"failed to wake worker set {worker_set}: {rep.p.error}")

        # and wait
        if timeout:
            await asyncio.wait_for(self._worker_healthy_waiters[bench_id].wait(), timeout)
        else:
            await self._worker_healthy_waiters[bench_id].wait()
        if bench_id in self._worker_healthy_waiters:
            del self._worker_healthy_waiters[bench_id]
        log.info("worker_observer.wait_until_healthy.done")

    async def run(self):
        logger.info("start")

        benches = await sync_to_async(_get_benches_to_manage)()
        self._worker_sets_by_bench_id = {
            ws.bench_id: ws
            async for ws in models.WorkerSet.objects.select_related(
                "bench", "bench__organization", "bench__user"
            ).all()
        }
        await asyncio.gather(*[self._prepare_runtime_host(bench.head_id) for bench in benches])

        logger.info("ready")
        self._ready = True

    @property
    def ready(self) -> bool:
        return self._ready

    @property
    def healthy(self):
        return self.ready and self.tasks.healthy

    async def _prepare_runtime_host(self, module_id: UUID) -> "RuntimeHost":
        runtime = self.runtimes.get(module_id)
        if runtime is None:
            logger.info("runtime.prepare", module_id=module_id)
            # start language worker if not already started
            module = await models.Module.objects.select_related(
                "bench", "bench__user", "bench__organization"
            ).aget(id=module_id)
            bench = module.parent_bench
            runtime = RuntimeHost(self.id, self.tasks, self, bench, module)
            self.runtimes[module_id] = runtime
            self.tasks.start(runtime.run(), f"worker-{module_id}")
        if not runtime.ready.is_set():
            await runtime.ready.wait()
        return runtime

    async def stop(self):
        logger.info("stop")
        self._ready = False

    async def run(self):
        await k8.init()
        logger.info("start")

        # load
        worker_sets = [
            ws
            async for ws in models.WorkerSet.objects.select_related(
                "bench", "bench__organization", "bench__user"
            ).all()
        ]
        for worker_set in worker_sets:
            self.worker_sets_by_bench_id[worker_set.bench_id] = worker_set
            self.worker_sets_by_id[worker_set.id] = worker_set
            # scale active worker sets target to desired
            if not worker_set.sleeping:
                worker_set.target_replicas = worker_set.desired_replicas
        logger.debug("workers.loaded_from_db", worker_sets=self.worker_sets)

        # initial sync
        if KUBERNETES_ENABLED:
            # fetch from k8
            k8_deployments, k8_revision_mark = await k8.get_all_deployments()
            k8_deployments_to_kill: list[k8.Deployment] = []
            active_worker_node_ids: list[str] = []
            dead_worker_node_ids: set[str] = set()
            for k8_deployment in k8_deployments:
                worker_set = self.worker_sets_by_bench_id.get(k8_deployment.bench_id)
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
            await self._mark_runs_dead(bench_id=None, worker_node_ids=dead_worker_node_ids)

        # start for real
        self.tasks.start(self._manage_worker_lifecycle_forever())
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
            benches = [worker_set.bench for worker_set in worker_sets]
            await k8.update_deployments(benches, deployments)
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
        worker_sets_data = [packer.pack_struct(worker_set) for worker_set in worker_sets]
        bench_id = worker_sets_data[0].bench_id if len(worker_sets) == 1 else None
        await publish(
            NMessageType.WORKERS_CHANGED,
            WorkersChangedPayload(bench_id=bench_id, worker_sets=worker_sets_data),
        )

    async def _mark_runs_dead(
        self,
        bench_id: Optional[UUID],
        worker_node_ids: Collection[str],
        run_ids: Optional[Collection[UUID]] = None,
    ):
        """Marks runs on worker nodes as aborted (node may be lost or just restarting)."""
        logger.debug("mark_runs_dead", worker_node_ids=worker_node_ids, run_ids=run_ids)

        if bench_id is not None:
            # extend worker_node_ids by any worker nodes that have an active run but aren't in k8
            worker_set = self.worker_sets_by_bench_id.get(bench_id)
            active_runs = models.Run.objects.filter(
                worker_node_id__isnull=False, status__in=ACTIVE_RUN_STATUSES
            ).values_list("worker_node_id", flat=True)
            also_dead_worker_node_ids = {
                i async for i in active_runs if i not in worker_set.active_replicas_ids
            }
            worker_node_ids = set(worker_node_ids) | also_dead_worker_node_ids

        # collect presumed dead runs
        dead_runs_qs = models.Run.objects.filter(
            Q(
                Q(worker_node_id__in=worker_node_ids) | Q(worker_node_id=None),
                status__in=ACTIVE_RUN_STATUSES,
            )
            | Q(id__in=run_ids or [])
        )
        if bench_id:
            dead_runs_qs = dead_runs_qs.filter(bench_id=bench_id)
        dead_runs: list[models.Run] = [r async for r in dead_runs_qs]

        # get bench search index names
        module_ids = set(r.module_id for r in dead_runs)
        module_pg_by_id: dict[UUID, models.Module] = {
            p.id: p async for p in models.Module.objects.filter(id__in=module_ids)
        }
        model_pgs = [module_pg_by_id[pv_id] for pv_id in module_ids]

        # mark dead and send out updates
        if not dead_runs:
            return
        for run in dead_runs:
            run.mark_dead()
        await models.Run.objects.abulk_update(dead_runs, ["status", "terminated_at"])
        dead_runs_data = [packer.pack_node_flat(r) for r in dead_runs]
        await sync_to_async(write_runs_to_os)(model_pgs, dead_runs_data)
        dead_runs_data_by_bench_id = group_by(dead_runs_data, lambda r: r.bench_id)
        for bench_id, dead_runs_data in dead_runs_data_by_bench_id.items():
            await publish(
                NMessageType.RUNS_CHANGED_GLOBAL, RunsChangedGlobalPayload(runs=dead_runs_data)
            )

    async def _watch_worker_sets_in_k8_forever(self, k8_marker: k8.VersionMarker):
        """Watch k8 deployments and update worker sets accordingly."""
        async for event_type, object in k8.watch_our_deployments(k8_marker):
            logger.debug("k8.event", event_type=event_type, object=object)
            worker_set = self.worker_sets_by_bench_id.get(object.bench_id)
            if worker_set is None:
                logger.warning("workers.unknown", object=object)
                continue  # delete maybe?

            if isinstance(object, k8.Deployment):
                # deployment / worker set changed
                partial_worker_set = object.to_model()
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
                    await self._mark_runs_dead(worker_set.bench_id, [object.name])
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
        tired_worker_sets = []
        idle_cutoff = utcnow_with_tz() - timedelta(seconds=WORKER_SET_IDLE_SLEEP_TIME)
        for worker_set in self.worker_sets:
            if worker_set.sleeping:
                continue
            # put to sleep if idle for too long
            if (
                worker_set.last_active_at is None or worker_set.last_active_at < idle_cutoff
            ) and not (worker_set.last_bumped_at and worker_set.last_bumped_at > idle_cutoff):
                worker_set.sleeping = True
                worker_set.target_replicas = 0
                tired_worker_sets.append(worker_set)
        logger.debug("workers.mark_tired", worker_sets=tired_worker_sets)
        return tired_worker_sets

    async def _get_bench_worker_set(self, bench_id: UUID):
        """Get worker set for a bench (load if not already loaded, may have just been created)."""
        worker_set = self._worker_sets_by_bench_id.get(bench_id)
        if worker_set is None:
            # newly created bench, get from DB
            bench = await models.Bench.objects.select_related(
                "worker_set",
                "worker_set__bench",
                "worker_set__bench__user",
                "worker_set__bench__organization",
            ).aget(id=bench_id)
            self._worker_sets_by_bench_id[bench_id] = bench.worker_set
            self._worker_sets_by_id[bench.worker_set.id] = bench.worker_set
            return bench.worker_set
        else:
            return worker_set

    async def configure_worker_set(self, msg: NMessage[ReqConfigureWorkerSetPayload]) -> None:
        try:
            logger.info("workers.configure", msg=msg)
            worker_set = await self._get_bench_worker_set(msg.p.bench_id)
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

    async def wake_worker_set(self, msg: NMessage[ReqWakeWorkerSetPayload]) -> None:
        try:
            logger.info("workers.wake", msg=msg)
            worker_set = await self._get_bench_worker_set(msg.p.bench_id)
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

    async def restart_worker_set(self, msg: NMessage[ReqRestartWorkerSetPayload]) -> None:
        worker_set = await self._get_bench_worker_set(msg.p.bench_id)
        logger.info("workers.restart", worker_set=worker_set)
        success = False

        # restart (if we have any nodes)
        dead_replicas_ids = worker_set.active_replicas_ids[:]  # may change during restart
        if worker_set.target_replicas > 0:
            try:
                # TODO @Robustness: do restart worker node only works with :1WorkerNode
                rep: NMessage[RepDoRestartWorkerNodePayload] = await request(
                    NMessageType.DO_RESTART_WORKER_NODE,
                    ReqDoRestartWorkerNodePayload(
                        bench_id=msg.p.bench_id,
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
            await self._mark_runs_dead(msg.p.bench_id, dead_replicas_ids)
        else:
            await self._mark_runs_dead(msg.p.bench_id, ["local"])

        await msg.reply(
            RepRestartWorkerSetPayload(
                worker_set_id=worker_set.id if worker_set else None, success=success
            )
        )
