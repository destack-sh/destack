import asyncio
from queue import Queue

from bench.models import Execution, ExecutionStatus
from bench.runtime.tracing import ExecutionFrame

# TODO @Cleanup: runtime.server should probably live in django-side of the backend


class DbExecutionTracker:
    """Server-side execution tracker that stores execution frames in the database."""

    def __init__(self):
        self.pending_frames_queue: Queue[ExecutionFrame] = asyncio.Queue()

    def receive_frame(self, frame: ExecutionFrame):
        self.pending_frames_queue.put_nowait(frame)

    def process_until_empty(self):
        # TODO @Performance: batch execution tracker updates
        while not self.pending_frames_queue.empty():
            self.process_one()

    def process_one(self):
        frame = self.pending_frames_queue.get()
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

    def join(self):
        self.pending_frames_queue.join()
