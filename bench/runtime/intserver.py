import asyncio

import structlog
from asgiref.sync import sync_to_async

from bench.models import Execution, ProjectVersion, mapper
from bench.models.mapper import read_module, write_module
from bench.msg import NMessage
from bench.msg.core import handle_reply, message_handler, nc_init, publish, subscribe
from bench.msg.messages import (
    ExecutionChangedPayload,
    ExecutionSavedPayload,
    ModuleChangedPayload,
    NMessageType,
    ProjectVersionChangedPayload,
    RepReadModulePayload,
    RepWriteModulePayload,
    ReqReadModulePayload,
    ReqWriteModulePayload,
)
from bench.msg.sync import is_semantic_mutation
from bench.runtime.type import ExecutionFrameData
from bench.utils.utils import sentry_capture_if_enabled

logger = structlog.get_logger(__name__)


class InternalServer:
    """
    Server-side Bench language server for reading and writing modules in DB.
    Can be thought of as a sidecar to the main API server for internal operations.
    """

    def __init__(self):
        self.subs = []

    async def run(self):
        await nc_init.wait()
        logger.info("start")
        self.subs = [
            await handle_reply(NMessageType.REQUEST_READ_MODULE, self.read_module),
            await handle_reply(NMessageType.REQUEST_WRITE_MODULE, self.write_module),
            await subscribe(f"{NMessageType.EXECUTION_CHANGED}.*", cb=self.execution_changed),
            await subscribe(
                f"{NMessageType.PROJECT_VERSION_CHANGED}.*", cb=self.project_version_changed
            ),
        ]

    @message_handler
    async def read_module(self, msg: NMessage[ReqReadModulePayload]) -> None:
        project_v = await ProjectVersion.objects.aget(id=msg.payload.module_id)
        module = await sync_to_async(read_module)(project_v)
        await msg.reply(RepReadModulePayload(module=module, project_id=project_v.project_id))

    @message_handler
    async def write_module(self, msg: NMessage[ReqWriteModulePayload]) -> None:
        logger.info("write_module", files=msg.payload.files, module_id=msg.payload.module_id)
        project_v = await ProjectVersion.objects.aget(id=msg.payload.module_id)
        try:
            if project_v.committed:
                raise ValueError(f"cannot write to committed {project_v}")
            await sync_to_async(write_module)(
                files=msg.payload.files,
                generated_mappings=msg.payload.generated_mappings,
                project_v=project_v,
                overwrite=True,
            )
            success = True
        except Exception as e:
            sentry_enabled = sentry_capture_if_enabled(e)
            logger.error("write_module", exc_info=e, sentry_enabled=sentry_enabled)
            success = False
        await msg.reply(RepWriteModulePayload(success=success))

    @message_handler
    async def execution_changed(self, msg: NMessage[ExecutionChangedPayload]) -> None:
        save_success = await sync_to_async(save_execution_frames)(msg.payload.frames)
        if save_success:
            # forward to API clients now that DB frames are saved
            await publish(
                NMessageType.EXECUTION_SAVED,
                ExecutionSavedPayload(
                    module_id=msg.p.project_version_id, execution_id=msg.p.execution_id
                ),
            )

    @message_handler
    async def project_version_changed(self, msg: NMessage[ProjectVersionChangedPayload]) -> None:
        # reload project version as module
        # TODO @Performance: send partial module updates :PartialModuleUpdates
        if not any(is_semantic_mutation(mutation) for mutation in msg.p.mutations):
            return  # ignore non-semantic changes to modules
        project_v = await ProjectVersion.objects.filter(id=msg.p.project_version_id).afirst()
        if project_v is None:
            return  # just ignore, was probably deleted
        module = await sync_to_async(read_module)(project_v)
        await publish(
            NMessageType.MODULE_CHANGED, ModuleChangedPayload(module_id=module.id, module=module)
        )

    async def stop(self):
        logger.info("stop")
        await asyncio.gather(sub.unsubscribe() for sub in self.subs)


def save_execution_frames(frames: list[ExecutionFrameData]) -> bool:
    model_executions: list[Execution] = []
    for frame in frames:
        execution = mapper.rmap_execution_frame(frame)
        model_executions.append(execution)

    try:
        # upsert frames
        Execution.objects.bulk_create(
            model_executions,
            update_conflicts=True,
            unique_fields=["id"],
            update_fields=["status", "terminated_at", "outputs", "error"],
        )
        logger.debug("save_execution_frames", executions=model_executions)
        return True
    except Exception as e:
        logger.error("save_execution_frames_failed", exc_info=e, executions=model_executions)
        return False
