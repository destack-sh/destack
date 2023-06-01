from __future__ import annotations

import contextvars
import typing
from collections import defaultdict
from dataclasses import dataclass
from datetime import datetime
from typing import Any
from uuid import UUID

import pytz
import structlog

from bench.language.type import Model, XBlock
from bench.language.typer import check_type
from bench.language.wire import ExecutionTracingLevel, ExecutionTriggerType
from bench.msg import NMessageType
from bench.msg.core import publish_soon
from bench.msg.messages import ExecutionChangedPayload
from bench.runtime.type import ExecutionFrame, ExecutionFrameData
from bench.utils.serialize import to_dict
from bench.utils.uuidt import UUIDT

if typing.TYPE_CHECKING:
    from bench.runtime.inference import Inference
    from bench.runtime.instance import CodeInstance, TaskInstance

logger = structlog.get_logger(__name__)


class Tracer:
    """
    Trace and track a session with executions, mutations, etc.
    """

    def queue_enter(self, code: CodeInstance, inputs: dict[str, Any], queue_position: int):
        pass

    def code_enter(self, code: CodeInstance, args, kwargs):
        pass

    def code_exit(self, code: CodeInstance, args, kwargs, result):
        pass

    def code_exception(self, code: CodeInstance, args, kwargs, exception: Exception):
        pass

    def inference_enter(self, model: Model, blocks: list[XBlock], settings: Any):
        pass

    def inference_exit(self, model: Model, blocks: list[XBlock], settings: Any, result: Any):
        pass

    def inference_exception(
        self, model: Model, blocks: list[XBlock], settings: Any, exception: Exception
    ):
        pass

    def inference_cached(
        self, model: Model, blocks: list[XBlock], settings: Any, inference: Inference
    ):
        pass


_context_tracers: contextvars.ContextVar[list[Tracer]] = contextvars.ContextVar(
    "tracers", default=[]
)
_all_tracers_blocked: contextvars.ContextVar[bool] = contextvars.ContextVar(
    "tracers_blocked", default=False
)


class MultiTracer(Tracer):
    """
    A worker-side tracer that delegates to multiple tracers.
    On exit, tracers are called in reverse order.
    Dynamic context tracers may be added with contextvars.
    """

    def __init__(self, tracers: list[Tracer]):
        self._static_tracers = tracers

    @property
    def tracers(self):
        if _all_tracers_blocked.get():
            return []
        tracers = self._static_tracers + _context_tracers.get()
        # ensure validation tracer is last
        # This is important because the ValidationTracer can throw in code_enter/code_exit,
        # so if it's not last, another tracer will exit first, then the validation tracer will error,
        # causing all tracers to be called _again_ for code_exception.
        tracers.sort(key=lambda t: isinstance(t, ValidationTracer))
        return tracers

    def queue_enter(self, code: CodeInstance, inputs: dict[str, Any], queue_position: int):
        for tracer in self.tracers:
            tracer.queue_enter(code, inputs, queue_position)

    def code_enter(self, code: CodeInstance, args, kwargs):
        for tracer in self.tracers:
            tracer.code_enter(code, args, kwargs)

    def code_exit(self, code: CodeInstance, args, kwargs, result):
        for tracer in reversed(self.tracers):
            tracer.code_exit(code, args, kwargs, result)

    def code_exception(self, code: CodeInstance, args, kwargs, exception: Exception):
        for tracer in reversed(self.tracers):
            try:
                tracer.code_exception(code, args, kwargs, exception)
            except Exception as e:
                # internal error in tracer, very bad
                logger.exception("trace.code.exception", exc_info=True, tracer=tracer)
                raise e

    def inference_enter(self, model: Model, blocks: list[XBlock], settings: Any):
        for tracer in self.tracers:
            tracer.inference_enter(model, blocks, settings)

    def inference_exit(self, model: Model, blocks: list[XBlock], settings: Any, result: Any):
        for tracer in reversed(self.tracers):
            tracer.inference_exit(model, blocks, settings, result)

    def inference_exception(
        self, model: Model, blocks: list[XBlock], settings: Any, exception: Exception
    ):
        for tracer in reversed(self.tracers):
            try:
                tracer.inference_exception(model, blocks, settings, exception)
            except Exception as e:
                # internal error in tracer, very bad
                logger.exception("trace.inference.exception", exc_info=True, tracer=tracer)
                raise e

    def inference_cached(
        self, model: Model, blocks: list[XBlock], settings: Any, inference: Inference
    ):
        for tracer in self.tracers:
            tracer.inference_cached(model, blocks, settings, inference)


def push_context_tracers(*tracers: Tracer):
    _context_tracers.set(_context_tracers.get() + list(tracers))
    return tracers


def pop_context_tracers(*tracers: Tracer, n: int | None = None):
    tracers = _context_tracers.get()[-n:] if n is not None else _context_tracers.get()
    _context_tracers.set([t for t in tracers if t not in _context_tracers.get()])


class BlockingTracerBoundary:
    """
    A context manager for blocking all tracers.
    """

    def __enter__(self):
        self.token = _all_tracers_blocked.set(True)

    def __exit__(self, exc_type, exc_val, exc_tb):
        _all_tracers_blocked.reset(self.token)


def tracer_blocker() -> BlockingTracerBoundary:
    """Block all tracers in the current context."""
    return BlockingTracerBoundary()


@dataclass(slots=True)
class WorkerContext:
    deployment_id: typing.Optional[UUID]
    module_id: UUID
    project_id: UUID
    worker_id: UUID


worker_ctx: contextvars.ContextVar[WorkerContext] = contextvars.ContextVar("worker_context")

ExecutionCapture = typing.Callable[[ExecutionFrame], None]

# global execution traces per execution tracer instance
_execution_stacktraces: contextvars.ContextVar[
    dict[int, list[ExecutionFrame]]
] = contextvars.ContextVar("execution_stacktraces")

# context manager for trace boundary


class TracerBoundary:
    def __enter__(self):
        self.token = _execution_stacktraces.set(defaultdict(list))

    def __exit__(self, exc_type, exc_val, exc_tb):
        _execution_stacktraces.reset(self.token)


def tracer_boundary():
    return TracerBoundary()


class ExecutionTracer(Tracer):
    """
    A worker-side tracer that records code and model executions.
    """

    _seq_id: typing.ClassVar[int] = 0

    def __init__(self, tracker: ExecutionCapture | None = None):
        self.tracker = tracker or (lambda frame: None)
        self._id = ExecutionTracer._seq_id
        ExecutionTracer._seq_id += 1

    def __del__(self):
        if _execution_stacktraces.get(None) is not None:
            del _execution_stacktraces.get()[self._id]

    def __str__(self):
        return str(self._id)

    def __repr__(self):
        return f"<ExecutionTracer {self._id}>"

    @property
    def stacktrace(self) -> list[ExecutionFrame]:
        return _execution_stacktraces.get()[self._id]

    def pop_stacktrace(self) -> ExecutionFrame:
        frame = self.stacktrace.pop()
        # update cached info in parent(s)
        if frame.cached_generated_at is not None:
            self._update_cached_info()
        return frame

    def _update_cached_info(self):
        for frame in self.stacktrace:
            frame.cached_generated_at = min(
                f.cached_generated_at for f in frame.walk_descendants() if f.cached_generated_at
            )
            frame.cached_duration = sum(
                f.cached_duration for f in frame.walk_descendants() if f.cached_duration
            )

    def _create_frame(
        self,
        runnable: typing.Optional[CodeInstance | TaskInstance | Model] = None,
        inputs: dict[str, Any] | None = None,
        queue_position: int | None = None,
        trace: bool = True,
    ):
        if trace:
            root = self.stacktrace[0] if self.stacktrace else None
            parent = self.stacktrace[-1] if self.stacktrace else None
        else:
            root = None
            parent = None
        frame = ExecutionFrame(
            id=UUIDT(),
            module_id=worker_ctx.get().module_id,
            runnable=runnable,
            root=root,
            parent=parent,
            entered_at=datetime.utcnow().replace(tzinfo=pytz.utc),
            exited_at=None,
            cached_generated_at=None,
            cached_duration=None,
            inputs=inputs,
            outputs=None,
            error=None,
            queue_position=queue_position,
        )
        if parent is not None:
            parent.children.append(frame)
        return frame

    def queue_enter(self, code: CodeInstance, inputs: dict[str, Any], queue_position: int):
        # don't trace this because it's not part of the stacktrace
        frame = self._create_frame(
            runnable=code, inputs=inputs, trace=False, queue_position=queue_position
        )
        self.tracker(frame)
        logger.debug("trace.queue", frame=frame)

    def code_enter(self, code: CodeInstance, args, kwargs):
        # map args into kwargs
        combined_kwargs = {**kwargs}
        for input_t, input in zip(code.inputs, args):
            combined_kwargs[input_t.name] = input
        frame = self._create_frame(
            runnable=code, inputs=code.type.rekey(combined_kwargs, is_output=False)
        )
        self.stacktrace.append(frame)
        self.tracker(frame)  # tracker may mutate/do other things, so log after it's run
        logger.debug("trace.code.enter", frame=frame, stackdepth=len(self.stacktrace))

    def code_exit(self, code: CodeInstance, args, kwargs, result):
        frame = self.pop_stacktrace()
        frame.exited_at = datetime.utcnow().replace(tzinfo=pytz.utc)
        frame.outputs = code.type.rekey(result, is_output=True)
        self.tracker(frame)
        logger.debug("trace.code.exit", frame=frame, stackdepth=len(self.stacktrace))

    def code_exception(self, code: CodeInstance, args, kwargs, exception: Exception):
        frame = self.pop_stacktrace()
        frame.exited_at = datetime.utcnow().replace(tzinfo=pytz.utc)
        frame.error = exception
        self.tracker(frame)
        logger.debug("trace.code.exception", frame=frame, stackdepth=len(self.stacktrace))

    def inference_enter(self, model: Model, blocks: list[XBlock], settings: Any):
        frame = self._create_frame(runnable=model)
        frame.inputs = [to_dict(block) for block in blocks]
        self.stacktrace.append(frame)
        self.tracker(frame)
        logger.debug("trace.inference.enter", frame=frame, stackdepth=len(self.stacktrace))

    def inference_exit(self, model: Model, blocks: list[XBlock], settings: Any, result: Any):
        frame = self.pop_stacktrace()
        frame.exited_at = datetime.utcnow().replace(tzinfo=pytz.utc)
        self.tracker(frame)
        logger.debug("trace.inference.exit", frame=frame, stackdepth=len(self.stacktrace))

    def inference_cached(
        self, model: Model, blocks: list[XBlock], settings: Any, inference: Inference
    ):
        # track a complete frame, don't add to stacktrace
        frame = self._create_frame(runnable=model, trace=True)
        frame.exited_at = datetime.utcnow().replace(tzinfo=pytz.utc)
        frame.cached_generated_at = inference.generated_at
        frame.cached_duration = inference.duration
        frame.inputs = [to_dict(block) for block in blocks]
        frame.outputs = inference.result
        self._update_cached_info()
        self.tracker(frame)
        logger.debug("trace.inference.cached", frame=frame, stackdepth=len(self.stacktrace))

    def inference_exception(
        self, model: Model, blocks: list[XBlock], settings: Any, exception: Exception
    ):
        frame = self.pop_stacktrace()
        frame.exited_at = datetime.utcnow().replace(tzinfo=pytz.utc)
        frame.error = exception
        self.tracker(frame)
        logger.debug("trace.inference.exception", frame=frame, stackdepth=len(self.stacktrace))


@dataclass(slots=True)
class ExecutionTrackerContext:
    # nocheckin: should this be generic session / tracing context?
    # trigger info and such should probably be in session anyway
    tracing_level: ExecutionTracingLevel
    trigger_type: ExecutionTriggerType
    trigger_id: typing.Optional[UUID]
    root_id: typing.Optional[UUID] = None


pub_tracker_ctx = contextvars.ContextVar("pub_tracker_context")


class PubExecutionTracker:
    def __call__(self, frame: ExecutionFrame):
        ctx = pub_tracker_ctx.get()
        trace_all_frames = ctx.tracing_level in (
            ExecutionTracingLevel.ALL_FRAMES,
            ExecutionTracingLevel.ALL_FRAMES_WITH_DATA,
        )
        trace_data = ctx.tracing_level in (
            ExecutionTracingLevel.ALL_FRAMES_WITH_DATA,
            ExecutionTracingLevel.ROOT_FRAME_WITH_DATA,
        )
        is_root = frame.root is None
        # filter according to trace level
        if not is_root and not trace_all_frames:
            return
        if is_root and ctx.root_id is not None:
            frame.id = ctx.root_id

        w = worker_ctx.get()
        frame_data = ExecutionFrameData.from_frame(
            frame,
            project_id=w.project_id,
            tracing_level=ctx.tracing_level,
            deployment_id=w.deployment_id,
            worker_id=w.worker_id,
            trigger_type=ctx.trigger_type,
            trigger_id=ctx.trigger_id,
        )

        # wipe data if not tracing it
        # TODO @Cleanup: consider not tracking untracked data at all when creating execution frame
        if not trace_data:
            frame_data.inputs = None
            frame_data.outputs = None

        logger.debug("execution.track", frame=frame_data.id)
        publish_soon(
            NMessageType.EXECUTION_CHANGED,
            ExecutionChangedPayload(frame.module_id, frames=[frame_data]),
        )


class InMemoryExecutionTracker:
    def __init__(self, root_only: bool = False):
        self.tracer = ExecutionTracer(self)
        self.frames: list[ExecutionFrame] = []
        self.root_only = root_only

    def __call__(self, frame: ExecutionFrame):
        if self.root_only and frame.root is not None:
            return
        if not any(f.id == frame.id for f in self.frames):
            self.frames.append(frame)

    def __enter__(self):
        push_context_tracers(self.tracer)
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        pop_context_tracers(self.tracer)

    @property
    def roots(self) -> list[ExecutionFrame]:
        return [f for f in self.frames if f.root is None]


def in_memory_traces() -> InMemoryExecutionTracker:
    return InMemoryExecutionTracker()


class ValidationError(Exception):
    pass


class ValidationTracer(Tracer):
    """
    A worker-side tracer that validates inputs and outputs.
    """

    def __init__(self, eager_validation: bool = True):
        """
        @param eager_validation: whether to bail on the first error or collect all errors
        """
        self.eager_validation = eager_validation

    def code_enter(self, code: CodeInstance, args, kwargs):
        try:
            combined_kwargs = {**kwargs}
            for input_t, input in zip(code.inputs, args):
                combined_kwargs[input_t.name] = input
            check_type(combined_kwargs, code, is_output=False)
        except (KeyError, ValueError, TypeError) as e:
            raise ValidationError(f"invalid arguments for {code.name}: {e}", e)

    def code_exit(self, code: CodeInstance, args, kwargs, result):
        try:
            check_type(result, code, is_output=True)
        except TypeError as e:
            raise ValidationError(f"invalid return value for {code.name}: {e}", e)
