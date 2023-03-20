from __future__ import annotations

import asyncio
import enum
import traceback
from dataclasses import dataclass, field
from datetime import datetime
from typing import Any, Callable, ClassVar, Coroutine, Optional
from uuid import UUID

from bench.language.parse import ModuleIndex
from bench.language.type import (
    Build,
    Code,
    Dataset,
    LiteralValue,
    Model,
    Module,
    SymbolType,
    Task,
    Type,
    Value,
)
from bench.language.wire import ExecutionTracingLevel, ExecutionTriggerType
from bench.utils.record import RecordBatch
from bench.utils.utils import required_field
from bench.utils.uuidt import UUIDT

AsyncCodeCallable = Callable[..., Coroutine]
SyncCodeCallable = Callable[..., Any]


@dataclass(repr=False)
class ModuleInstance:
    module: Module
    index: ModuleIndex


@dataclass(repr=False)
class SymbolInstance:
    build: Optional[Build] = None

    @property
    def symbol_type(self):
        return SYMBOL_TYPE_BY_INSTANCE_CLASS[self.__class__]

    @property
    def py_handle(self) -> Any:
        raise NotImplementedError


@dataclass(repr=False)
class TaskInstance(SymbolInstance, Task):
    code: CodeInstance = required_field()

    @property
    def py_handle(self) -> Any:
        return self.code.py_handle


@dataclass(repr=False)
class TypeInstance(SymbolInstance, Type):
    py_type: Any = required_field()

    @property
    def py_handle(self) -> Any:
        return self.py_type


@dataclass(repr=False)
class DatasetInstance(SymbolInstance, Dataset):
    records_batch: RecordBatch = required_field()

    @property
    def py_handle(self) -> RecordBatch:
        return self.records_batch


@dataclass(repr=False)
class ValueInstance(SymbolInstance, Value):
    @property
    def py_handle(self):
        return self.value


@dataclass(repr=False)
class ModelInstance(SymbolInstance, Model):
    @property
    def py_handle(self):
        return self


@dataclass(repr=False)
class CodeInstance(SymbolInstance, Code):
    task: Optional[TaskInstance] = None
    transformed_code: str = required_field()
    code_callable: SyncCodeCallable | AsyncCodeCallable = required_field()
    is_async: bool = required_field()

    @property
    def py_handle(self) -> SyncCodeCallable | AsyncCodeCallable:
        return self.code_callable


SYMBOL_TYPE_BY_INSTANCE_CLASS = {
    TaskInstance: SymbolType.TASK,
    TypeInstance: SymbolType.TYPE,
    DatasetInstance: SymbolType.DATA,
    ValueInstance: SymbolType.VALUE,
    ModelInstance: SymbolType.MODEL,
    CodeInstance: SymbolType.CODE,
}


@dataclass
class ExecutionFrame:
    id: UUID
    module_id: UUID
    build: Optional[Build]
    task: Optional[TaskInstance]
    code: Optional[CodeInstance]
    model: Optional[ModelInstance]
    root: Optional[ExecutionFrame]
    parent: Optional[ExecutionFrame]
    inference_id: Optional[UUID]
    entered_at: datetime
    exited_at: Optional[datetime]
    inputs: Optional[dict[str, LiteralValue]]
    outputs: Optional[LiteralValue]
    error: Optional[Exception]
    queue_position: Optional[int]

    def __str__(self):
        # get str of all non-null fields
        fields_strs = [
            f"module={self.module_id}",
            f"build={self.build}" if self.build else None,
            f"task={self.task}" if self.task else None,
            f"code={self.code}" if self.code else None,
            f"model={self.model}" if self.model else None,
            f"root={self.root.id}" if self.root else None,
            f"parent={self.parent.id}" if self.parent else None,
            f"inference_context={self.inference_id}" if self.inference_id else None,
            f"entered={self.entered_at}",
            f"exited={self.exited_at}" if self.exited_at else None,
            f"inputs={summarize_args(self.inputs)}",
            f"outputs={summarize_args(self.outputs)}" if self.outputs else None,
            f"error={self.error}" if self.error else None,
            f"queue_position={self.queue_position}" if self.queue_position else None,
        ]
        fields_str = [s for s in fields_strs if s]
        return f"id={self.id} ({', '.join(fields_str)})"

    def __repr__(self):
        return f"<ExecutionFrame {self}>"


@dataclass
class ErrorData:
    """Wire-able representation of an exception."""

    type: str
    message: str
    traceback: list[str]


@dataclass
class ExecutionFrameData:
    """Wire-able representation of an execution frame."""

    id: UUID
    module_id: UUID
    build_id: Optional[UUID]
    task_id: Optional[UUID]
    code_id: Optional[UUID]
    model_id: Optional[UUID]
    root_id: Optional[UUID]
    parent_id: Optional[UUID]
    inference_id: Optional[UUID]
    entered_at: datetime
    exited_at: Optional[datetime]
    inputs: dict[str, Any]
    outputs: Optional[Any]
    error: Optional[ErrorData]
    queue_position: Optional[int]
    # additional context data not in ExecutionFrame
    project_id: UUID
    tracing_level: Optional[ExecutionTracingLevel]
    deployment_id: UUID
    trigger_type: Optional[ExecutionTriggerType]
    trigger_id: Optional[UUID]

    @staticmethod
    def from_frame(
        frame: ExecutionFrame,
        *,
        project_id: UUID,
        tracing_level: ExecutionTracingLevel,
        deployment_id: UUID,
        trigger_type: Optional[ExecutionTriggerType] = None,
        trigger_id: Optional[UUID] = None,
    ) -> ExecutionFrameData:
        if frame.error:
            error_data = ErrorData(
                type=type(frame.error).__name__,
                message=str(frame.error),
                traceback=traceback.format_exception(
                    type(frame.error), frame.error, frame.error.__traceback__
                ),
            )
        else:
            error_data = None
        return ExecutionFrameData(
            id=frame.id,
            module_id=frame.module_id,
            build_id=frame.build.id if frame.build else None,
            task_id=frame.task.id if frame.task else None,
            code_id=frame.code.id if frame.code else None,
            model_id=frame.model.id if frame.model else None,
            root_id=frame.root.id if frame.root else None,
            parent_id=frame.parent.id if frame.parent else None,
            entered_at=frame.entered_at,
            exited_at=frame.exited_at,
            inputs=frame.inputs,
            outputs=frame.outputs,
            inference_id=frame.inference_id,
            error=error_data,
            queue_position=frame.queue_position,
            project_id=project_id,
            tracing_level=tracing_level,
            deployment_id=deployment_id,
            trigger_type=trigger_type,
            trigger_id=trigger_id,
        )


class FinishReason(enum.StrEnum):
    MAX_TOKENS = "max_tokens"
    STOP = "stop"


@dataclass
class TextGeneration:
    text: str
    tokens: Optional[list[str]]
    logits: Optional[list[float]]
    finish_reason: FinishReason


def summarize_args(arguments: Any) -> str:
    """
    Summarize the names (if available) and types of arguments.
    """
    if isinstance(arguments, dict):
        return ", ".join(f"{name}={type(value).__name__}" for name, value in arguments.items())
    elif isinstance(arguments, (list, tuple, set)):
        return ", ".join(type(value).__name__ for value in arguments)
    else:
        return type(arguments).__name__


class JobType(enum.StrEnum):
    INTERP = "interp"
    GENERATE = "generate"
    BUILD = "build"
    EVALUATE = "evaluate"
    LINT = "lint"
    RUN = "run"


class JobStatus(enum.StrEnum):
    Queued = "queued"
    Running = "running"
    Completed = "completed"
    Cancelling = "cancelling"
    Cancelled = "cancelled"
    Failed = "failed"


# lower is higher
JOB_DEFAULT_PRIORITY = {
    JobType.INTERP: 0,
    JobType.RUN: 0,
    JobType.GENERATE: 1,
    JobType.BUILD: 2,
    JobType.EVALUATE: 3,
    JobType.LINT: 4,
}


@dataclass(repr=False)
class Job:
    type: ClassVar[JobType]
    project_id: UUID
    project_version_id: UUID
    deployment_id: Optional[UUID]
    id: UUID = field(default_factory=UUIDT)
    status: JobStatus = JobStatus.Queued
    started_at: Optional[datetime] = None
    terminated_at: Optional[datetime] = None
    task: Optional[asyncio.Task] = None
    terminated: asyncio.Event = field(default_factory=asyncio.Event)

    def __str__(self):
        return f"{self.type} {self.id} ({self.status})"

    def __repr__(self):
        return f"<Job {self}>"

    def __lt__(self, other):
        return self.default_priority < other.default_priority

    @property
    def success(self) -> bool:
        raise NotImplementedError

    @property
    def default_priority(self) -> int:
        return JOB_DEFAULT_PRIORITY[self.type]


@dataclass(repr=False)
class JobData:
    project_id: UUID
    project_version_id: UUID
    deployment_id: Optional[UUID]
    id: UUID
    type: JobType
    status: JobStatus
    started_at: Optional[datetime]
    terminated_at: Optional[datetime]

    @staticmethod
    def from_job(job: Job) -> JobData:
        return JobData(
            type=job.type,
            id=job.id,
            status=job.status,
            started_at=job.started_at,
            terminated_at=job.terminated_at,
        )
