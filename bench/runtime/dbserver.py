import structlog
import zmq.asyncio
from asgiref.sync import sync_to_async

from bench.models import Execution, ExecutionStatus, ProjectVersion
from bench.models.mapper import read_module
from bench.runtime.tracing import ExecutionFrame
from bench.zmq import ZMessage, ZMessageType, recv_message_poll, send_message, zmq_ctx
from bench.zmq.messages import (
    ProjectVersionChangedPayload,
    RepReadModulePayload,
    ReqReadModulePayload,
)

# TODO @Cleanup: dbservers should probably live in django-side of the backend?
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
        self.change_sub_sock = zmq_ctx.socket(zmq.SUB)
        self.change_pub_sock = zmq_ctx.socket(zmq.PUB)

    async def start(self, internal_server_addr: str, api_server_addr: str):
        logger.info(
            "internal_server.start",
            internal_server_addr=internal_server_addr,
            api_server_addr=api_server_addr,
        )
        self.rep_sock.bind(internal_server_addr)
        self.change_sub_sock.connect(api_server_addr)
        self.change_pub_sock.bind(internal_server_addr)

        poller = zmq.asyncio.Poller()
        poller.register(self.rep_sock, zmq.POLLIN)
        poller.register(self.change_sub_sock, zmq.POLLIN)

        while True:
            msg = await recv_message_poll(poller)
            await self.process_message(msg)

    async def process_message(self, msg: ZMessage) -> None:
        logger.debug("internal_server.process", request=msg)
        if msg.type == ZMessageType.REQ_READ_MODULE:
            # get module from DB
            module_id = msg.payload_as(ReqReadModulePayload).module_id
            project_v = await ProjectVersion.objects.aget(id=module_id)
            module = await sync_to_async(read_module)(project_v)
            send_message(
                self.rep_sock,
                ZMessage(ZMessageType.REP_READ_MODULE, RepReadModulePayload(module=module)),
            )
        elif msg.type == ZMessageType.PROJECT_VERSION_CHANGED:
            # reload project version as module
            # TODO @Performance: send partial module updates instead of full reloads
            module_id = msg.payload_as(ProjectVersionChangedPayload).project_version_id
            project_v = await ProjectVersion.objects.aget(id=module_id)
            module = await sync_to_async(read_module)(project_v)
            send_message(
                self.change_pub_sock,
                ZMessage(ZMessageType.REP_READ_MODULE, RepReadModulePayload(module=module)),
            )
        else:
            raise ValueError(f"unexpected message: {msg}")

    async def stop(self):
        logger.info("internal_server.stop")
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
        code_id=frame.code.definition.id,
        model_id=frame.model.definition.id if frame.model else None,
        defaults=dict(
            started_at=frame.entered_at,
            terminated_at=frame.exited_at,
            status=status,
            inputs=frame.inputs,
            outputs=frame.outputs,
            error=error,
        ),
    )
