from __future__ import annotations

import typing
from dataclasses import dataclass

from bench.language.parse import ModuleIndex
from bench.language.type import (
    Code,
    Dataset,
    Expectation,
    Model,
    Module,
    StatementPath,
    Task,
    Type,
    Value,
)
from bench.utils.record import RecordBatch

AsyncCodeCallable = typing.Callable[..., typing.Coroutine]
SyncCodeCallable = typing.Callable[..., typing.Any]


@dataclass(repr=False)
class InstantiatedModule:
    module: Module
    index: ModuleIndex
    instances_by_path: dict[StatementPath, StatementInstance]


@dataclass
class StatementInstance:
    pass


@dataclass
class SymbolInstance(StatementInstance):
    @property
    def py_handle(self) -> typing.Any:
        raise NotImplementedError


@dataclass(repr=False)
class TypeInstance(Type, SymbolInstance):
    py_type: typing.Any

    @property
    def py_handle(self) -> typing.Any:
        return self.py_type


@dataclass(repr=False)
class TaskInstance(Task, SymbolInstance):
    pass


@dataclass(repr=False)
class ExpectationInstance(Expectation, SymbolInstance):
    pass


@dataclass(repr=False)
class DatasetInstance(Dataset, SymbolInstance):
    records_batch: RecordBatch

    @property
    def py_handle(self) -> RecordBatch:
        return self.records_batch


@dataclass(repr=False)
class ValueInstance(Value, SymbolInstance):
    @property
    def py_handle(self):
        return self.value


@dataclass(repr=False)
class ModelInstance(Model, SymbolInstance):
    @property
    def py_handle(self):
        return self


@dataclass(repr=False)
class CodeInstance(Code, SymbolInstance):
    transformed_code: str
    code_callable: SyncCodeCallable | AsyncCodeCallable
    prompt: typing.Optional[DynamicPrompt]

    @property
    def py_handle(self) -> SyncCodeCallable | AsyncCodeCallable:
        return self.code_callable


@dataclass
class DynamicPrompt:
    python_code: str
    settings: "PromptSettings"


@dataclass(slots=True)
class DecoderSettings:
    temperature: float
    max_tokens: int
    stop: list[str]


@dataclass(slots=True)
class PromptSettings:
    model: ModelInstance
    temperature: float
    max_tokens: int
    stop: list[str]


@dataclass
class TextGeneration:
    text: str
    tokens: list[str]
    logits: list[float]
