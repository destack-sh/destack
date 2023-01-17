import structlog
import zmq

from bench.models import Execution, ExecutionStatus
from bench.runtime.tracing import ExecutionFrame
from bench.utils.zmq import ZMessage, recv_message, send_message, zmq_ctx

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

    async def start(self, internal_server_addr: str):
        logger.info("internal_server.start", internal_server_addr=internal_server_addr)
        # request/reply for read/write modules
        self.rep_sock.bind(internal_server_addr)

        while True:
            request = await recv_message(self.rep_sock)

            response = await self.handle_request(request)
            await send_message(self.rep_sock, response)

    async def handle_request(self, request: ZMessage) -> ZMessage:
        logger.debug("internal_server.handle", request=request)
        raise NotImplementedError

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
