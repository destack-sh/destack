import asyncio
from datetime import datetime, timedelta
from uuid import UUID

import pytz
import structlog

from bench import models
from bench.msg import nc_init
from bench.msg.core import NMessage, handle_reply, message_handler
from bench.msg.messages import (
    NMessageType,
    RepRegisterWorkerNodePayload,
    ReqConfigureWorkerSetPayload,
    ReqRegisterWorkerPayload,
    ReqRestartWorkerNodePayload,
    ReqWakeWorkerSetPayload,
)
from bench.utils.func import wrap_task
from bench.utils.utils import sentry_capture_if_enabled
from bench.utils.uuidt import UUIDT

logger = structlog.get_logger(__name__)


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
        await nc_init.wait()
        logger.info("start")
        self.subs = [
            await handle_reply(NMessageType.CONFIGURE_WORKER_SET, self.configure_worker_set),
            await handle_reply(NMessageType.REGISTER_WORKER_NODE, self.register_worker_node),
            await handle_reply(NMessageType.WAKE_WORKER_SET, self.wake_worker_set),
            await handle_reply(NMessageType.RESTART_WORKER_NODE, self.restart_worker_node),
        ]
        self.tasks = [
            create_wrapped_task(self.sync_worker_nodes(interval_seconds=10)),
            create_wrapped_task(self.manage_timeouts(interval_seconds=10, timeout_seconds=60)),
        ]

    @message_handler
    async def register_worker_node(self, msg: NMessage[ReqRegisterWorkerPayload]) -> None:
        try:
            raise NotImplementedError  # nocheckin
            success = True
            logger.info("register_worker_node", node=node)
        except Exception as e:
            sentry_capture_if_enabled(e)
            logger.error("register_worker_node.failed", msg=msg, exc_info=True)
            success = False
        await msg.reply(RepRegisterWorkerNodePayload(success=success))

    @message_handler
    async def configure_worker_set(self, msg: NMessage[ReqConfigureWorkerSetPayload]) -> None:
        try:
            raise NotImplementedError  # nocheckin
            success = True
            logger.info("configure_worker_set", node=node)
        except Exception as e:
            sentry_capture_if_enabled(e)
            logger.error("configure_worker_set.failed", msg=msg, exc_info=True)
            success = False
        await msg.reply(RepConfigureWorkerSetPayload(success=success))

    @message_handler
    async def wake_worker_set(self, msg: NMessage[ReqWakeWorkerSetPayload]) -> None:
        raise NotImplementedError

    @message_handler
    async def restart_worker_node(self, msg: NMessage[ReqRestartWorkerNodePayload]) -> None:
        try:
            raise NotImplementedError  # nocheckin
            success = True
            logger.info("configure_worker_set", node=node)
        except Exception as e:
            sentry_capture_if_enabled(e)
            logger.error("configure_worker_set.failed", msg=msg, exc_info=True)
            success = False
        await msg.reply(RepConfigureWorkerSetPayload(success=success))

    async def _wake_project_if_needed(self, project_id: UUID):
        raise NotImplementedError  # nocheckin

    async def sync_worker_nodes(self, interval_seconds: int):
        """Update last seens and mark any unresponsive workers as inactive."""
        while True:
            # get last seen for all workers

            # check if there are any dead workers

            # mark all relevant jobs and runs as failed

            await asyncio.sleep(interval_seconds)

    async def manage_timeouts(self, interval_seconds: int, timeout_seconds: int):
        """Mark any timed out jobs or runs as failed."""
        while True:
            start_cutoff = datetime.utcnow().replace(tzinfo=pytz.utc) - timedelta(
                seconds=timeout_seconds
            )
            raise NotImplementedError  # nocheckin
