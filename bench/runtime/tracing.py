from __future__ import annotations

import contextvars
import copy
import typing
from dataclasses import dataclass
from datetime import datetime
from itertools import chain
from typing import Any
from uuid import UUID

import pytz
import structlog

from bench.language.type import XBlock
from bench.language.typer import check_type
from bench.language.wire import ExecutionTracingLevel, ExecutionTriggerType
from bench.msg import NMessageType
from bench.msg.core import publish_soon
from bench.msg.messages import ExecutionChangedPayload
from bench.runtime.model import InferenceContext
from bench.runtime.type import CodeInstance, ExecutionFrame, ExecutionFrameData, ModelInstance
from bench.utils.uuidt import UUIDT

logger = structlog.get_logger(__name__)


class Tracer:
    """
    A worker-side tracer that can be attached to an execution.
    Tracer will be called in an async context on a worker pod.
    """

    def code_enter(self, code: CodeInstance, args, kwargs):
        pass

    def code_exit(self, code: CodeInstance, args, kwargs, result):
        pass

    def code_exception(self, code: CodeInstance, args, kwargs, exception: Exception):
        pass

    def inference_enter(self, ctx: InferenceContext, blocks: list[XBlock], settings: Any):
        pass

    def inference_exit(
        self, ctx: InferenceContext, blocks: list[XBlock], settings: Any, result: Any
    ):
        pass

    def inference_exception(
        self, ctx: InferenceContext, blocks: list[XBlock], settings: Any, exception: Exception
    ):
        pass


class MultiTracer(Tracer):
    """
    A worker-side tracer that delegates to multiple tracers.
    On exit, tracers are called in reverse order.
    """

    def __init__(self, tracers: list[Tracer]):
        self.tracers = tracers

    def code_enter(self, code: CodeInstance, args, kwargs):
        for tracer in self.tracers:
            tracer.code_enter(code, args, kwargs)

    def code_exit(self, code: CodeInstance, args, kwargs, result):
        for tracer in reversed(self.tracers):
            tracer.code_exit(code, args, kwargs, result)

    def code_exception(self, code: CodeInstance, args, kwargs, exception: Exception):
        for tracer in reversed(self.tracers):
            tracer.code_exception(code, args, kwargs, exception)

    def inference_enter(self, ctx: InferenceContext, blocks: list[XBlock], settings: Any):
        for tracer in self.tracers:
            tracer.inference_enter(ctx, blocks, settings)

    def inference_exit(
        self, ctx: InferenceContext, blocks: list[XBlock], settings: Any, result: Any
    ):
        for tracer in reversed(self.tracers):
            tracer.inference_exit(ctx, blocks, settings, result)

    def inference_exception(
        self, ctx: InferenceContext, blocks: list[XBlock], settings: Any, exception: Exception
    ):
        for tracer in reversed(self.tracers):
            tracer.inference_exception(ctx, blocks, settings, exception)


_context_tracers: contextvars.ContextVar[list[Tracer]] = contextvars.ContextVar(
    "tracers", default=[]
)


def push_context_tracers(*tracers: Tracer):
    _context_tracers.set(_context_tracers.get() + list(tracers))


def pop_context_tracers(*tracers: Tracer):
    _context_tracers.set([t for t in tracers if t not in _context_tracers.get()])


class ContextTracer(MultiTracer):
    """A tracer that uses dynamic tracers from context variables (and the predefined tracers)."""

    @property
    def tracers(self):
        return chain(_context_tracers.get(), super().tracers)


class Trace:
    """A trace collected by a worker-side tracer."""

    pass


@dataclass(slots=True)
class WorkerContext:
    deployment_id: UUID
    module_id: UUID
    project_id: UUID
    worker_id: UUID


worker: contextvars.ContextVar[WorkerContext] = contextvars.ContextVar("worker_context")

ExecutionCapture = typing.Callable[[ExecutionFrame], None]


@dataclass
class ExecutionTrace(Trace):
    frames: list[ExecutionFrame]

    @property
    def root(self) -> typing.Optional[ExecutionFrame]:
        return self.frames[0] if self.frames else None


class ExecutionTracer(Tracer):
    """
    A worker-side tracer that records code and model executions.
    """

    def __init__(self, tracker: ExecutionCapture | None = None):
        self.tracker = tracker or (lambda frame: None)
        self.stacktraces: contextvars.ContextVar[list[ExecutionFrame]] = contextvars.ContextVar(
            "stacktraces", default=[]
        )

    @property
    def stacktrace(self) -> list[ExecutionFrame]:
        return self.stacktraces.get()

    @property
    def trace(self) -> ExecutionTrace:
        return self.traces.get()[-1]

    def _create_frame(
        self,
        code: typing.Optional[CodeInstance] = None,
        model: typing.Optional[ModelInstance] = None,
        inference_context: typing.Optional[InferenceContext] = None,
        inputs: dict[str, Any] | None = None,
        trace: bool = True,
        queue_position: int | None = None,
    ):
        root = self.stacktrace[0] if self.stacktrace else None
        parent = self.stacktrace[-1] if self.stacktrace else None
        frame = ExecutionFrame(
            id=UUIDT(),
            module_id=worker.get().module_id,
            build=code.build if code else parent.build,  # keep build if root had it?
            task=code.task if code else None,
            code=code,
            model=model,
            root=root,
            parent=parent,
            entered_at=datetime.utcnow().replace(tzinfo=pytz.utc),
            exited_at=None,
            inputs=inputs,
            outputs=None,
            inference_id=inference_context.id if inference_context else None,
            error=None,
            queue_position=queue_position,
        )
        if trace:
            self.trace.frames.append(frame)
        return frame

    def queue_enter(self, code: CodeInstance, inputs: dict[str, Any], queue_position: int):
        # don't trace this because it's not part of the stacktrace
        frame = self._create_frame(
            code=code, inputs=inputs, trace=False, queue_position=queue_position
        )
        self.tracker(frame)
        logger.debug("trace.queue", frame=frame)

    def code_enter(self, code: CodeInstance, args, kwargs):
        inputs = {
            **copy.deepcopy(kwargs),
            **{f"__arg_{i}": (i, copy.deepcopy(arg)) for i, arg in args},
        }
        frame = self._create_frame(code=code, inputs=inputs)
        self.stacktrace.append(frame)
        self.tracker(frame)  # tracker may mutate/do other things, so log afterwards
        logger.debug("trace.code.enter", frame=frame, stackdepth=len(self.stacktrace))

    def code_exit(self, code: CodeInstance, args, kwargs, result):
        frame = self.stacktrace.pop()
        frame.exited_at = datetime.utcnow().replace(tzinfo=pytz.utc)
        frame.outputs = copy.deepcopy(result)
        self.tracker(frame)
        logger.debug("trace.code.exit", frame=frame, stackdepth=len(self.stacktrace))

    def code_exception(self, code: CodeInstance, args, kwargs, exception: Exception):
        frame = self.stacktrace.pop()
        frame.exited_at = datetime.utcnow().replace(tzinfo=pytz.utc)
        frame.error = exception
        self.tracker(frame)
        logger.debug("trace.code.exception", frame=frame, stackdepth=len(self.stacktrace))

    def inference_enter(self, ctx: InferenceContext, blocks: list[XBlock], settings: Any):
        frame = self._create_frame(model=ctx.model, inference_context=ctx)
        self.stacktrace.append(frame)
        self.tracker(frame)
        logger.debug("trace.inference.enter", frame=frame, stackdepth=len(self.stacktrace))

    def inference_exit(
        self, ctx: InferenceContext, blocks: list[XBlock], settings: Any, result: Any
    ):
        frame = self.stacktrace.pop()
        frame.exited_at = datetime.utcnow().replace(tzinfo=pytz.utc)
        self.tracker(frame)
        logger.debug("trace.inference.exit", frame=frame, stackdepth=len(self.stacktrace))

    def inference_exception(
        self, ctx: InferenceContext, blocks: list[XBlock], settings: Any, exception: Exception
    ):
        frame = self.stacktrace.pop()
        frame.exited_at = datetime.utcnow().replace(tzinfo=pytz.utc)
        frame.error = exception
        self.tracker(frame)
        logger.debug("trace.inference.exception", frame=frame, stackdepth=len(self.stacktrace))


@dataclass(slots=True)
class PubTrackerContext:
    tracing_level: ExecutionTracingLevel
    trigger_type: ExecutionTriggerType
    trigger_id: UUID
    root_id: typing.Optional[UUID] = None


pub_tracker_context = contextvars.ContextVar("pub_tracker_context")


class PubExecutionTracker:
    def __call__(self, frame: ExecutionFrame):
        ctx = pub_tracker_context.get()
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

        w = worker.get()
        frame_data = ExecutionFrameData.from_frame(
            frame,
            project_id=w.module_id,
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


class ValidationError(Exception):
    pass


class ValidationTracer(Tracer):
    """
    A worker-side tracer that validates inputs and outputs.
    """

    def __init__(self, eager_validation: bool = False):
        """
        @param eager_validation: whether to bail on the first error or collect all errors
        """
        self.eager_validation = eager_validation

    def code_enter(self, code: CodeInstance, args, kwargs):
        try:
            # check args
            for i, value in enumerate(args):
                value_type = code.type_node.input.children[i]
                check_type(value, value_type, eager_error=self.eager_validation)
            # check kwargs
            for name, value in kwargs.items():
                value_type = code.type_node.input.child(name)
                check_type(value, value_type, eager_error=self.eager_validation)
        except (KeyError, ValueError, TypeError) as e:
            raise ValidationError(f"invalid arguments for {code.name}: {e}", e)

    def code_exit(self, code: CodeInstance, args, kwargs, result):
        try:
            check_type(result, code.type_node.output)
        except TypeError as e:
            raise ValidationError(f"invalid return value for {code.name}: {e}", e)
