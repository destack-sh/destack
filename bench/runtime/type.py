from __future__ import annotations

import enum
import traceback
from dataclasses import dataclass
from datetime import datetime
from typing import Any, Callable, Coroutine, Optional
from uuid import UUID

from bench.language.parse import ModuleIndex
from bench.language.type import (
    Code,
    Dataset,
    LiteralValue,
    Model,
    Module,
    SymbolType,
    Type,
    Value,
)
from bench.utils.record import RecordBatch
from bench.utils.utils import required_field

AsyncCodeCallable = Callable[..., Coroutine]
SyncCodeCallable = Callable[..., Any]


@dataclass(repr=False)
class ModuleInstance:
    module: Module
    index: ModuleIndex


@dataclass
class SymbolInstance:
    @property
    def symbol_type(self):
        return SYMBOL_TYPE_BY_INSTANCE_CLASS[self.__class__]

    @property
    def py_handle(self) -> Any:
        raise NotImplementedError


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
    transformed_code: str = required_field()
    code_callable: SyncCodeCallable | AsyncCodeCallable = required_field()
    is_async: bool = required_field()
    prompt: Optional[DynamicPrompt] = required_field()

    @property
    def py_handle(self) -> SyncCodeCallable | AsyncCodeCallable:
        return self.code_callable


SYMBOL_TYPE_BY_INSTANCE_CLASS = {
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
    code: Optional[CodeInstance]
    model: Optional[ModelInstance]
    root: Optional[ExecutionFrame]
    parent: Optional[ExecutionFrame]
    inference_context_id: Optional[UUID]
    entered_at: datetime
    exited_at: Optional[datetime]
    inputs: Optional[dict[str, LiteralValue]]
    outputs: Optional[LiteralValue]
    error: Optional[Exception]

    def __str__(self):
        # get str of all non-null fields
        fields_strs = [
            f"module={self.module_id}",
            f"code={self.code}" if self.code else None,
            f"model={self.model}" if self.model else None,
            f"root={self.root.id}" if self.root else None,
            f"parent={self.parent.id}" if self.parent else None,
            f"inference_context={self.inference_context_id}" if self.inference_context_id else None,
            f"entered={self.entered_at}",
            f"exited={self.exited_at}" if self.exited_at else None,
            f"inputs={summarize_args(self.inputs)}",
            f"outputs={summarize_args(self.outputs)}" if self.outputs else None,
            f"error={self.error}" if self.error else None,
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
    code_id: Optional[UUID]
    model_id: Optional[UUID]
    root_id: Optional[UUID]
    parent_id: Optional[UUID]
    inference_context_id: Optional[UUID]
    entered_at: datetime
    exited_at: Optional[datetime]
    inputs: dict[str, LiteralValue]
    outputs: Optional[LiteralValue]
    error: Optional[ErrorData]

    @staticmethod
    def from_frame(frame: ExecutionFrame) -> ExecutionFrameData:
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
            code_id=frame.code.id if frame.code else None,
            model_id=frame.model.id if frame.model else None,
            root_id=frame.root.id if frame.root else None,
            parent_id=frame.parent.id if frame.parent else None,
            entered_at=frame.entered_at,
            exited_at=frame.exited_at,
            inputs=frame.inputs,
            outputs=frame.outputs,
            inference_context_id=frame.inference_context_id,
            error=error_data,
        )


@dataclass
class DynamicPrompt:
    python_code: str
    settings: "PromptSettings"


@dataclass(slots=True)
class DecoderSettings:
    temperature: float
    max_tokens: int
    stop: list[str] | None

    def __post_init__(self):
        # max tokens must be > 0
        if self.max_tokens <= 0:
            raise ValueError("max_tokens must be greater than 0")
        # temperature must be [0, 1]
        if self.temperature < 0 or self.temperature > 1:
            raise ValueError("temperature must be between 0 and 1")


@dataclass(slots=True)
class PromptSettings:
    model: ModelInstance
    temperature: float
    max_tokens: int
    max_generated_tokens: int
    stop: list[str] | None


class FinishReason(enum.StrEnum):
    MAX_TOKENS = "max_tokens"
    STOP = "stop"


@dataclass
class TextGeneration:
    text: str
    tokens: list[str]
    logits: list[float]
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
