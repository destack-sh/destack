import asyncio
from uuid import UUID

import structlog
from django.db import transaction

from bench import models
from bench.language.session import WorkerProfile, WorkerRegion, WorkerSetStatus
from bench.msg import nc_init
from bench.msg.core import NMessage, handle_reply, message_handler
from bench.msg.messages import (
    NMessageType,
    RepConfigureWorkerSetPayload,
    ReqConfigureWorkerSetPayload,
    ReqRestartWorkerNodePayload,
    ReqWakeWorkerSetPayload,
)
from bench.server import k8
from bench.server.k8 import K8_AVAILABLE
from bench.utils.func import wrap_task
from bench.utils.utils import DEBUG, sentry_capture_if_enabled
from bench.utils.uuidt import UUIDT

logger = structlog.get_logger(__name__)

USE_K8 = K8_AVAILABLE or not DEBUG


def create_wrapped_task(coro, task_id: str = None):
    asyncio.create_task(wrap_task(coro, task_id))


class MasterServer:
    """
    Server to orchestrate workers in k8s requests.
    There can only be one master globally for now, which is "enforced" by the deployment config.
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

    async def _update_worker_sets_in_k8(self, worker_sets: list[models.WorkerSet]) -> None:
        deployments = [k8.Deployment.from_model(worker_set) for worker_set in worker_sets]
        await k8.update_deployments(deployments)

    async def _get_project_worker_set(self, project_id: UUID):
        """Get or create a default worker set for a project."""
        worker_set = self.worker_sets_by_project_id.get(project_id)
        if worker_set is None:
            with transaction.atomic():
                worker_set = await models.WorkerSet.objects.acreate(
                    project_id=project_id,
                    region=WorkerRegion.EU_CENTRAL,
                    profile=WorkerProfile.TINY,
                    target_count=1,
                    actual_count=0,
                    status=WorkerSetStatus.PENDING,
                )
                project = await models.Project.objects.aget(id=project_id)
                project.worker_set = worker_set
                await project.asave()
        return worker_set

    async def _wake_worker_set(self, project_id: UUID) -> models.WorkerSet:
        worker_set = await self._get_project_worker_set(project_id)
        if worker_set.status != WorkerSetStatus.RUNNING:
            worker_set.status = WorkerSetStatus.CREATING
            await worker_set.asave()
            await self._update_worker_sets_in_k8([worker_set])

    async def _wake_project_if_needed(self, project_id: UUID):
        worker_set = await self._get_project_worker_set(project_id)
        if worker_set.status != WorkerSetStatus.RUNNING:
            await self._update_worker_sets_in_k8([worker_set])

    @message_handler
    async def configure_worker_set(self, msg: NMessage[ReqConfigureWorkerSetPayload]) -> None:
        try:
            project = await models.Project.objects.select_related("worker_set").aget(
                id=msg.p.project_id
            )
            worker_set = await self._update_worker_sets_in_k8([project.worker_set])
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
                project = await models.Project.objects.select_related("worker_set").aget(
                    id=msg.p.project_id
                )
                worker_set = await self._wake_worker_set(project.id)
                logger.info("wake_worker_set", worker_set=worker_set)
            success = True
        except Exception as e:
            sentry_capture_if_enabled(e)
            logger.error("wake_worker_set.failed", msg=msg, exc_info=True)
            success = False
        await msg.reply(RepConfigureWorkerSetPayload(success=success))

    @message_handler
    async def restart_worker_set(self, msg: NMessage[ReqRestartWorkerNodePayload]) -> None:
        try:
            if not USE_K8:
                # touch 'manage.py' file to trigger reload in dev mode
                with open("manage.py", "a"):
                    pass
            else:
                project = await models.Project.objects.select_related("worker_set").aget(
                    id=msg.p.project_id
                )
                worker_set = await self._wake_worker_set(project.id)
                await k8.restart_deployment(k8.Deployment.from_model(worker_set))
                logger.info("restart_worker_set", worker_set=worker_set)
            success = True
        except Exception as e:
            sentry_capture_if_enabled(e)
            logger.error("restart_worker_set.failed", msg=msg, exc_info=True)
            success = False
        await msg.reply(RepConfigureWorkerSetPayload(success=success))
