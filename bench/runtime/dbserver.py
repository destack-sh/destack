import asyncio

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

    async def orchestrate(self):
        raise NotImplementedError


class InternalServer:
    """Server-side Bench language server for reading and writing modules in DB."""

    async def handle_request(self, request: ZMessage) -> ZMessage:
        raise NotImplementedError

    async def serve(self, port=5555):
        logger.info("start_internal_server", port=port)
        # request/reply for read/write modules
        async with zmq_ctx.socket(zmq.REP) as sock:
            sock.bind(f"tcp://*:{port}")

            while True:
                request = await recv_message(sock)
                logger.info("received request", request=request)

                response = await self.handle_request(request)
                await send_message(sock, response)


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
