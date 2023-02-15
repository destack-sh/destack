import structlog
import zmq.asyncio
from asgiref.sync import sync_to_async

from bench.models import Execution, ExecutionStatus, ProjectVersion
from bench.models.mapper import read_module, write_module
from bench.runtime.tracing import ExecutionFrame
from bench.zmq import ZMessage, ZMessageType, recv_message_poll, send_message, zmq_ctx
from bench.zmq.messages import (
    ModuleChangedPayload,
    ProjectVersionChangedPayload,
    RepReadModulePayload,
    RepWriteModulePayload,
    ReqReadModulePayload,
    ReqWriteModulePayload,
)

# TODO @Cleanup: intservers should probably live in django-side of the backend?
#  (not general language runtime)

logger = structlog.get_logger(__name__)


class WorkerOrchestrator:
    """Worker orchestrator manages workers lifecycles (incl. heartbeats)"""

    async def start(self, port: int):
        raise NotImplementedError


class InternalServer:
    """Server-side Bench language server for reading and writing modules in DB."""

    def __init__(self):
        self.rep_sock = zmq_ctx.socket(zmq.REP)
        self.sub_sock = zmq_ctx.socket(zmq.SUB)
        self.pub_sock = zmq_ctx.socket(zmq.PUB)

    async def run(
        self, internal_server_rep_addr: str, internal_server_pub_addr: str, api_server_pub_addr: str
    ):
        logger.info(
            "start",
            internal_server_rep_addr=internal_server_rep_addr,
            internal_server_pub_addr=internal_server_pub_addr,
            api_server_pub_addr=api_server_pub_addr,
        )
        self.rep_sock.bind(internal_server_rep_addr)
        self.sub_sock.connect(api_server_pub_addr)
        self.sub_sock.setsockopt(zmq.SUBSCRIBE, b"")
        self.pub_sock.bind(internal_server_pub_addr)

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
        elif msg.type == ZMessageType.PROJECT_VERSION_CHANGED:
            # reload project version as module
            # TODO @Performance: send partial module updates :PartialModuleUpdates
            module_id = msg.payload_as(ProjectVersionChangedPayload).project_version_id
            project_v = await ProjectVersion.objects.aget(id=module_id)
            module = await sync_to_async(read_module)(project_v)
            send_message(
                self.pub_sock,
                ZMessageType.MODULE_CHANGED,
                ModuleChangedPayload(module_id=module.id, module=module),
            )
        elif msg.type == ZMessageType.REQ_WRITE_MODULE:
            write = msg.payload_as(ReqWriteModulePayload)
            logger.info("write_module", files=write.files, module_id=write.module_id)
            project_v = await ProjectVersion.objects.aget(id=write.module_id)
            try:
                await sync_to_async(write_module)(write.files, project_v, overwrite=True)
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
        else:
            raise ValueError(f"unexpected message: {msg}")

    async def stop(self):
        logger.info("stop")
        self.rep_sock.close()


async def store_inbound_execution_frames():
    # TODO @Incomplete: forward execution frames from zmq to execution tracker
    raise NotImplementedError


async def save_execution_frame(frame: ExecutionFrame):
    if frame.exited_at:
        status = ExecutionStatus.Completed
    elif frame.exception:
        status = ExecutionStatus.Failed
    else:
        status = ExecutionStatus.Running

    if frame.exception:
        error = {
            "message": str(frame.exception),
            "type": type(frame.exception).__name__,
        }
    else:
        error = None

    await Execution.objects.aupdate_or_create(
        id=frame.id,
        parent_id=frame.parent.id if frame.parent else None,
        code_id=frame.code.source.id,
        model_id=frame.model.source.id if frame.model else None,
        defaults=dict(
            started_at=frame.entered_at,
            terminated_at=frame.exited_at,
            status=status,
            inputs=frame.inputs,
            outputs=frame.outputs,
            error=error,
        ),
    )
