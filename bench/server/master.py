import asyncio
from datetime import datetime
import time
from uuid import UUID

import structlog

from bench import models
from bench.language.session import WorkerProfile, WorkerRegion, WorkerSetStatus
from bench.models import packer
from bench.models.worker import WORKER_SET_FIELDS
from bench.msg import nc_init
from bench.msg.core import NMessage, handle_reply, message_handler, publish
from bench.msg.messages import (
    NMessageType,
    RepConfigureWorkerSetPayload,
    ReqConfigureWorkerSetPayload,
    ReqRestartWorkerNodePayload,
    ReqWakeWorkerSetPayload,
    WorkersChangedPayload,
)
from bench.server import k8
from bench.server.k8 import K8_AVAILABLE
from bench.utils.cache import redis
from bench.utils.func import wrap_task
from bench.utils.utils import DEBUG, sentry_capture_if_enabled
from bench.utils.uuidt import UUIDT

logger = structlog.get_logger(__name__)

USE_K8 = K8_AVAILABLE or not DEBUG
WORKER_SET_IDLE_SLEEP_TIME = 60 * 20  # 20 minutes


def create_wrapped_task(coro, task_id: str = None):
    asyncio.create_task(wrap_task(coro, task_id))


class MasterServer:
    """
    Server to orchestrate workers in k8s requests.
    There can only be one master globally for now, which is "enforced" by deploying as StatefulSet.
    TODO @Robustness: use k8 lease for master server election
    """

    def __init__(self):
        self.id = UUIDT()
        self.subs = []
        self.tasks = []
        self.worker_sets_by_project_id: dict[UUID, models.WorkerSet] = {}

    async def run(self):
        await k8.init()
        await nc_init.wait()
        logger.info("start")
        self.subs = [
            await handle_reply(NMessageType.CONFIGURE_WORKER_SET, self.configure_worker_set),
            await handle_reply(NMessageType.WAKE_WORKER_SET, self.wake_worker_set),
            await handle_reply(NMessageType.RESTART_WORKER_SET, self.restart_worker_set),
        ]
        async for worker_set in models.WorkerSet.objects.all():
            self.worker_sets_by_project_id[worker_set.project_id] = worker_set
        if USE_K8:
            self.tasks.append(asyncio.create_task(self._watch_worker_sets_in_k8()))
        self.tasks.append(asyncio.create_task(self._manage_worker_sets_sleep()))

    async def _update_worker_sets_in_k8(self, worker_sets: list[models.WorkerSet]) -> None:
        deployments = [k8.Deployment.from_model(worker_set) for worker_set in worker_sets]
        await k8.update_deployments(deployments)

    async def _update_worker_sets(self, worker_sets: list[models.WorkerSet]) -> None:
        if not worker_sets:
            return
        await models.WorkerSet.objects.abulk_update(worker_sets, update_fields=WORKER_SET_FIELDS)
        await self._update_worker_sets_in_k8(worker_sets)
        await publish(
            NMessageType.WORKERS_CHANGED,
            WorkersChangedPayload(
                project_id=worker_sets[0].project_id if len(worker_sets) == 1 else None,
                worker_sets=[packer.pack_data(worker_set) for worker_set in worker_sets],
            ),
        )

    async def _watch_worker_sets_in_k8(self):
        """Watch k8 deployments and update worker sets accordingly."""
        async for event_type, deployment in k8.watch_our_deployments():
            partial_worker_set = deployment.to_model()
            worker_set = self.worker_sets_by_project_id.get(partial_worker_set.project_id)
            if worker_set is None:
                logger.warning("worker_set.unknown", worker_set=partial_worker_set)
                continue  # delete maybe?
            for field in k8.DEPLOYMENT_DYNAMIC_FIELDS:
                setattr(worker_set, field, getattr(partial_worker_set, field))
            logger.info("worker_set.update", worker_set=worker_set)
            await worker_set.asave(force_update=True, update_fields=k8.DEPLOYMENT_DYNAMIC_FIELDS)
            await publish(
                NMessageType.WORKERS_CHANGED,
                WorkersChangedPayload(
                    project_id=worker_set.project_id,
                    worker_sest=[packer.pack_data(worker_set)],
                ),
            )

    async def _manage_worker_sets_sleep(self, interval: int = 60):
        """Puts worker sets to sleep after inactivity if needed."""
        while True:
            await asyncio.sleep(interval)
            idle_cutoff = time.time() - WORKER_SET_IDLE_SLEEP_TIME
            # scan iter "worker_set.{id}" in redis
            keys = await redis.keys("worker_set.*.last_active_at")
            values = await redis.mget(keys)
            worker_sets = []
            worker_sets_to_sleep = []
            for key, last_active_at in zip(keys, values):
                worker_set_id = UUID(key.split(".")[1])
                last_active_at = float(last_active_at)
                worker_set = self.worker_sets_by_project_id.get(worker_set_id)
                if worker_set is None:
                    continue
                worker_sets.append(worker_set)
                worker_set.last_active_at = datetime.fromtimestamp(last_active_at)
                if worker_set.status != WorkerSetStatus.HEALTHY:
                    await redis.delete(key)
                    continue
                if last_active_at < idle_cutoff:
                    worker_sets_to_sleep.append(worker_set)

            logger.info("worker_sets.last_active_at", worker_sets=worker_sets)
            await models.WorkerSet.objects.abulk_update(
                worker_sets, update_fields=["last_active_at"]
            )
            if worker_sets_to_sleep:
                await self._sleep_worker_sets(worker_sets_to_sleep)

    async def _sleep_worker_sets(self, worker_sets: list[models.WorkerSet]):
        logger.info("worker_sets.sleep", worker_sets=worker_sets)
        for worker_set in worker_sets:
            worker_set.sleeping = True
            worker_set.target_replicas = 0
            worker_set.status = WorkerSetStatus.SLEEPING

    async def _wake_worker_sets(self, worker_sets: list[models.WorkerSet]):
        logger.info("worker_sets.wake", worker_sets=worker_sets)
        for worker_set in worker_sets:
            worker_set.sleeping = False
            worker_set.target_replicas = worker_set.desired_replicas
            worker_set.status = WorkerSetStatus.PENDING
        await self._update_worker_sets(worker_sets)

    async def _get_or_create_project_worker_set(self, project_id: UUID):
        """Get or create a default worker set for a project."""
        worker_set = self.worker_sets_by_project_id.get(project_id)
        if worker_set is None:
            worker_set = await models.WorkerSet.objects.acreate(
                project_id=project_id,
                region=WorkerRegion.EU_CENTRAL,
                profile=WorkerProfile.TINY,
                sleeping=False,
                desired_replicas=1,
                target_replicas=1,
                status=WorkerSetStatus.PENDING,
            )
            project = await models.Project.objects.aget(id=project_id)
            project.worker_set = worker_set
            await project.asave()
        return worker_set

    @message_handler
    async def configure_worker_set(self, msg: NMessage[ReqConfigureWorkerSetPayload]) -> None:
        try:
            project = await models.Project.objects.select_related("worker_set").aget(
                id=msg.p.project_id
            )
            worker_set = await self._get_or_create_project_worker_set(project.id)
            await self._update_worker_sets([worker_set])
            success = True
            logger.info("configure_worker_set", worker_set=worker_set)
        except Exception as e:
            sentry_capture_if_enabled(e)
            logger.error("configure_worker_set.failed", msg=msg, exc_info=True)
            success = False
        await msg.reply(RepConfigureWorkerSetPayload(success=success))

    @message_handler
    async def wake_worker_set(self, msg: NMessage[ReqWakeWorkerSetPayload]) -> None:
        try:
            if USE_K8:
                worker_set = await self._get_or_create_project_worker_set(msg.p.project_id)
                await self._wake_worker_sets([worker_set])
                logger.info("wake_worker_set", worker_set=worker_set)
            success = True
        except Exception as e:
            sentry_capture_if_enabled(e)
            logger.error("wake_worker_set.failed", msg=msg, exc_info=True)
            success = False
        await msg.reply(RepConfigureWorkerSetPayload(success=success))

    @message_handler
    async def restart_worker_set(self, msg: NMessage[ReqRestartWorkerNodePayload]) -> None:
        success = False
        try:
            if not USE_K8:
                # touch 'manage.py' file to trigger reload in dev mode
                with open("manage.py", "a"):
                    pass
            else:
                worker_set = await self._get_or_create_project_worker_set(msg.p.project_id)
                if worker_set.status == WorkerSetStatus.HEALTHY:
                    await k8.restart_deployment(k8.Deployment.from_model(worker_set))
                    success = True
                logger.info("restart_worker_set", worker_set=worker_set)
        except Exception as e:
            sentry_capture_if_enabled(e)
            logger.error("restart_worker_set.failed", msg=msg, exc_info=True)
        await msg.reply(RepConfigureWorkerSetPayload(success=success))
