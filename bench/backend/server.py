import asyncio

from bench.backend.tracing import ExecutionFrame
from bench.models import Execution, ExecutionStatus


class ModelExecutionTracker:
    """Server-side execution tracker that stores execution frames in the database."""

    def __init__(self):
        self.pending_frames_queue: asyncio.Queue[ExecutionFrame] = asyncio.Queue()

    def receive_frame(self, frame: ExecutionFrame):
        self.pending_frames_queue.put_nowait(frame)

    async def process_until_empty(self):
        # TODO @Performance: batch execution tracker updates
        while not self.pending_frames_queue.empty():
            await self.process_one()

    async def process_one(self):
        frame = await self.pending_frames_queue.get()
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
        self.pending_frames_queue.task_done()

    async def join(self):
        await self.pending_frames_queue.join()
