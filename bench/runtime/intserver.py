import structlog
import zmq.asyncio
from asgiref.sync import sync_to_async

from bench.models import Execution, ProjectVersion, mapper
from bench.models.mapper import read_module, write_module
from bench.msg import ZMessage, ZMessageType, recv_message_poll, send_message, zmq_ctx
from bench.msg.messages import (
    ExecutionChangedPayload,
    ModuleChangedPayload,
    ProjectVersionChangedPayload,
    RepReadModulePayload,
    RepWriteModulePayload,
    ReqReadModulePayload,
    ReqWriteModulePayload,
    as_key,
)
from bench.msg.sync import is_semantic
from bench.runtime.type import ExecutionFrameData

# TODO @Cleanup: intservers should probably live in django-side of the backend?
#  (not general language runtime)

logger = structlog.get_logger(__name__)


class InternalServer:
    """Server-side Bench language server for reading and writing modules in DB."""

    def __init__(self):
        self.rep_sock = zmq_ctx.socket(zmq.REP)
        self.sub_sock = zmq_ctx.socket(zmq.SUB)
        self.pub_sock = zmq_ctx.socket(zmq.PUB)

    async def run(
        self,
        intserver_rep_addr: str,
        intserver_pub_addr: str,
        api_pub_addr: str,
        worker_pub_addr: str,
    ):
        logger.info(
            "start",
            intserver_rep_addr=intserver_rep_addr,
            intserver_pub_addr=intserver_pub_addr,
            api_pub_addr=api_pub_addr,
            worker_pub_addr=worker_pub_addr,
        )
        self.rep_sock.bind(intserver_rep_addr)
        self.sub_sock.connect(api_pub_addr)
        self.sub_sock.connect(worker_pub_addr)
        self.sub_sock.setsockopt(zmq.SUBSCRIBE, as_key(ZMessageType.EXECUTION_CHANGED))
        self.sub_sock.setsockopt(zmq.SUBSCRIBE, as_key(ZMessageType.PROJECT_VERSION_CHANGED))
        self.pub_sock.bind(intserver_pub_addr)

        poller = zmq.asyncio.Poller()
        poller.register(self.rep_sock, zmq.POLLIN)
        poller.register(self.sub_sock, zmq.POLLIN)

        while True:
            msg = await recv_message_poll(poller)
            await self.process_message(msg)

    async def process_message(self, msg: ZMessage) -> None:
        logger.debug("process_message", request=msg)
        if msg.type == ZMessageType.REQ_READ_MODULE:
            # get module from DB
            module_id = msg.payload_as(ReqReadModulePayload).module_id
            project_v = await ProjectVersion.objects.aget(id=module_id)
            module = await sync_to_async(read_module)(project_v)
            send_message(
                self.rep_sock,
                ZMessageType.REP_READ_MODULE,
                RepReadModulePayload(module=module),
            )
        elif msg.type == ZMessageType.REQ_WRITE_MODULE:
            write: ReqWriteModulePayload = msg.payload_as(ReqWriteModulePayload)
            logger.info("write_module", files=write.files, module_id=write.module_id)
            project_v = await ProjectVersion.objects.aget(id=write.module_id)
            try:
                if project_v.committed:
                    raise ValueError(f"cannot write to committed {project_v}")
                await sync_to_async(write_module)(
                    files=write.files,
                    generated_mappings=write.generated_mappings,
                    project_v=project_v,
                    overwrite=True,
                )
                success = True
            except Exception as e:
                logger.error("write_module_failed", exc_info=e)
                success = False
            send_message(
                self.rep_sock, ZMessageType.REP_WRITE_MODULE, RepWriteModulePayload(success=success)
            )
            # notify module changed :PartialModuleUpdates
            module = await sync_to_async(read_module)(project_v)
            send_message(
                self.pub_sock,
                ZMessageType.MODULE_CHANGED,
                ModuleChangedPayload(module_id=module.id, module=module),
            )
        elif msg.type == ZMessageType.PROJECT_VERSION_CHANGED:
            # reload project version as module
            # TODO @Performance: send partial module updates :PartialModuleUpdates
            change: ProjectVersionChangedPayload = msg.payload_as(ProjectVersionChangedPayload)
            if not any(is_semantic(mutation) for mutation in change.mutations):
                return  # ignore non-semantic changes to modules
            project_v = await ProjectVersion.objects.filter(id=change.project_version_id).afirst()
            if project_v is None:
                return  # just ignore, was probably deleted
            module = await sync_to_async(read_module)(project_v)
            send_message(
                self.pub_sock,
                ZMessageType.MODULE_CHANGED,
                ModuleChangedPayload(module_id=module.id, module=module),
            )
        elif msg.type == ZMessageType.EXECUTION_CHANGED:
            changed: ExecutionChangedPayload = msg.payload_as(ExecutionChangedPayload)
            await sync_to_async(save_execution_frames)(changed.frames)
        else:
            raise ValueError(f"unexpected message: {msg}")

    async def stop(self):
        logger.info("stop")
        self.rep_sock.close()


def save_execution_frames(frames: list[ExecutionFrameData]):
    model_executions: list[Execution] = []
    for frame in frames:
        execution = mapper.rmap_execution_frame(frame)
        model_executions.append(execution)

    # upsert
    Execution.objects.bulk_create(
        model_executions,
        update_conflicts=True,
        unique_fields=["id"],
        update_fields=["status", "terminated_at", "outputs", "error"],
    )
    logger.debug("save_execution_frame", executions=model_executions)
