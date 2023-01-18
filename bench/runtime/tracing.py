from __future__ import annotations

import abc
import copy
import typing
import uuid
from dataclasses import dataclass
from datetime import datetime
from typing import Any

import pytz
import structlog

from bench.language.type import TypeNode
from bench.language.typer import derive_type_from_value
from bench.runtime.provider import Completion
from bench.runtime.type import CodeInstance, ModelInstance

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

    def model_complete_enter(self, model: ModelInstance, prompt: str):
        pass

    def model_complete_exit(
        self, model: ModelInstance, prompt: str, completion: Completion, inference_id: uuid.UUID
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

    def model_complete_enter(self, model: ModelInstance, prompt: str):
        for tracer in self.tracers:
            tracer.model_complete_enter(model, prompt)

    def model_complete_exit(
        self, model: ModelInstance, prompt: str, completion: Completion, inference_id: uuid.UUID
    ):
        for tracer in reversed(self.tracers):
            tracer.model_complete_exit(model, prompt, completion, inference_id)


class Trace:
    """A trace collected by a worker-side tracer."""

    pass


@dataclass
class ExecutionFrame:
    id: uuid.UUID
    code: CodeInstance
    model: typing.Optional[ModelInstance]
    parent: typing.Optional[ExecutionFrame]
    entered_at: datetime
    exited_at: typing.Optional[datetime]
    inputs: dict[str, Any]
    outputs: typing.Optional[dict[str, Any]]
    inference_id: typing.Optional[uuid.UUID]
    exception: typing.Optional[Exception]


class ExecutionTracker(abc.ABC):
    def __call__(self, frame: ExecutionFrame):
        raise NotImplementedError


@dataclass
class ExecutionTrace(Trace):
    frames: list[ExecutionFrame]

    @property
    def root(self) -> typing.Optional[ExecutionFrame]:
        return self.frames[0] if self.frames else None


class ExecutionTracer(Tracer):
    """
    A worker-side tracer that records the execution of a code.
    """

    def __init__(self, tracker: ExecutionTracker, trace: ExecutionTrace):
        self.tracker = tracker
        self.stacktrace: list[ExecutionFrame] = []
        self.trace = trace

    def _create_frame(
        self,
        code: typing.Optional[CodeInstance],
        model: typing.Optional[ModelInstance],
        inputs: dict[str, Any],
    ):
        parent = self.stacktrace[-1] if self.stacktrace else None
        frame = ExecutionFrame(
            id=uuid.uuid4(),
            code=code,
            model=model,
            parent=parent,
            entered_at=datetime.utcnow().replace(tzinfo=pytz.utc),
            exited_at=None,
            inputs=inputs,
            outputs=None,
            inference_id=None,
            exception=None,
        )
        self.trace.frames.append(frame)
        return frame

    def code_enter(self, code: CodeInstance, args, kwargs):
        inputs = {**copy.deepcopy(kwargs), "__args__": copy.deepcopy(args)}
        frame = self._create_frame(code, None, inputs)
        self.stacktrace.append(frame)
        self.tracker(frame)
        logger.debug("trace.code.enter", frame=frame, stackdepth=len(self.stacktrace))

    def code_exit(self, code: CodeInstance, args, kwargs, result):
        frame = self.stacktrace.pop()
        frame.exited_at = datetime.utcnow().replace(tzinfo=pytz.utc)
        self.tracker(frame)
        logger.debug("trace.code.exit", frame=frame, stackdepth=len(self.stacktrace))

    def code_exception(self, code: CodeInstance, args, kwargs, exception: Exception):
        frame = self.stacktrace.pop()
        frame.exited_at = datetime.utcnow().replace(tzinfo=pytz.utc)
        frame.exception = exception
        self.tracker(frame)
        logger.debug("trace.code.exception", frame=frame, stackdepth=len(self.stacktrace))

    def model_complete_enter(self, model: ModelInstance, prompt: str):
        # warn if there is no code on the stack
        parent_code = self.stacktrace[-1].code if self.stacktrace else None
        if parent_code is None:
            logger.warning(
                "trace.model.complete.enter.missing_parent", model=model, prompt=len(prompt)
            )

        inputs = {"prompt": prompt}
        frame = self._create_frame(parent_code, model, inputs)
        self.stacktrace.append(frame)
        self.tracker(frame)
        logger.debug("trace.model.complete.enter", frame=frame, stackdepth=len(self.stacktrace))

    def model_complete_exit(
        self, model: ModelInstance, prompt: str, completion: Completion, inference_id: uuid.UUID
    ):
        frame = self.stacktrace.pop()
        frame.exited_at = datetime.utcnow().replace(tzinfo=pytz.utc)
        # duplicating model inference in execution trace is not ideal but okay for now
        frame.outputs = {"completion": completion}
        frame.inference_id = inference_id
        self.tracker(frame)
        logger.debug("trace.model.complete.exit", frame=frame, stackdepth=len(self.stacktrace))


class ValidationError(Exception):
    pass


class ValidationTracer(Tracer):
    """
    A worker-side tracer that validates inputs and outputs.
    """

    def code_enter(self, code: CodeInstance, args, kwargs):
        # validate kwargs
        for name, value in kwargs.items():
            parameter = code.type_node.input.child(name)
            if parameter is None:
                # TODO @Typing: error on unknown parameters?
                #  Currently we ignore this because schema elements don't include non-value types.
                # ignore unknown parameters for now
                continue
            self._check_argument(code, value, parameter)

    def code_exit(self, code: CodeInstance, args, kwargs, result):
        self._check_output(code, result, code.type_node.output)

    def _check_output(self, code: CodeInstance, value: Any, type: TypeNode):
        # TODO @Typing: recursive schema validation
        value_type = derive_type_from_value(value)
        if not type.required and value is None:
            return
        elif value_type != type.type:
            raise ValidationError(f"return from {code} expected {type}, got {value_type}")

    def _check_argument(self, code: CodeInstance, argument: Any, parameter: TypeNode):
        # TODO @Typing: check that the argument has a compatible schema
        raise NotImplementedError
