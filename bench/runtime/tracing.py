from __future__ import annotations

import copy
import typing
import uuid
from dataclasses import dataclass
from datetime import datetime
from typing import Any

import pytz
import structlog

from bench.language.type import XBlock
from bench.language.typer import check_type
from bench.runtime.model import InferenceContext
from bench.runtime.type import CodeInstance, ExecutionFrame, ModelInstance
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

    def inference_enter(self, ctx: InferenceContext, blocks: tuple[XBlock]):
        pass

    def inference_exit(self, ctx: InferenceContext, blocks: tuple[XBlock], result: Any):
        pass

    def inference_exception(
        self, ctx: InferenceContext, blocks: tuple[XBlock], exception: Exception
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

    def inference_enter(self, ctx: InferenceContext, blocks: tuple[XBlock]):
        for tracer in self.tracers:
            tracer.inference_enter(ctx, blocks)

    def inference_exit(self, ctx: InferenceContext, blocks: tuple[XBlock], result: Any):
        for tracer in reversed(self.tracers):
            tracer.inference_exit(ctx, blocks, result)

    def inference_exception(
        self, ctx: InferenceContext, blocks: tuple[XBlock], exception: Exception
    ):
        for tracer in reversed(self.tracers):
            tracer.inference_exception(ctx, blocks, exception)


class Trace:
    """A trace collected by a worker-side tracer."""

    pass


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

    def __init__(
        self, module_id: uuid.UUID, tracker: ExecutionCapture, trace: ExecutionTrace | None = None
    ):
        self.module_id = module_id
        self.tracker = tracker
        self.stacktrace: list[ExecutionFrame] = []
        self.trace = trace or ExecutionTrace(frames=[])

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
            module_id=self.module_id,
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

    def inference_enter(self, ctx: InferenceContext, blocks: tuple[XBlock]):
        frame = self._create_frame(model=ctx.model, inference_context=ctx)
        self.stacktrace.append(frame)
        self.tracker(frame)
        logger.debug("trace.inference.enter", frame=frame, stackdepth=len(self.stacktrace))

    def inference_exit(self, ctx: InferenceContext, blocks: tuple[XBlock], result: Any):
        frame = self.stacktrace.pop()
        frame.exited_at = datetime.utcnow().replace(tzinfo=pytz.utc)
        self.tracker(frame)
        logger.debug("trace.inference.exit", frame=frame, stackdepth=len(self.stacktrace))

    def inference_exception(
        self, ctx: InferenceContext, blocks: tuple[XBlock], exception: Exception
    ):
        frame = self.stacktrace.pop()
        frame.exited_at = datetime.utcnow().replace(tzinfo=pytz.utc)
        frame.error = exception
        self.tracker(frame)
        logger.debug("trace.inference.exception", frame=frame, stackdepth=len(self.stacktrace))


class ValidationError(Exception):
    pass


class ValidationTracer(Tracer):
    """
    A worker-side tracer that validates inputs and outputs.
    """

    def code_enter(self, code: CodeInstance, args, kwargs):
        try:
            # check args
            for i, value in enumerate(args):
                value_type = code.type_node.input.children[i]
                check_type(value, value_type)
            # check kwargs
            for name, value in kwargs.items():
                value_type = code.type_node.input.child(name)
                check_type(value, value_type)
        except (KeyError, ValueError, TypeError) as e:
            raise ValidationError(f"invalid arguments for {code.name}: {e}", e)

    def code_exit(self, code: CodeInstance, args, kwargs, result):
        try:
            check_type(result, code.type_node.output)
        except TypeError as e:
            raise ValidationError(f"invalid return value for {code.name}: {e}", e)
