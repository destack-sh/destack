from __future__ import annotations

import typing
from dataclasses import dataclass

from bench.language.parse import ModuleIndex
from bench.language.type import (
    CodeContent,
    DatasetContent,
    ExpectationContent,
    ModelContent,
    Module,
    StatementPath,
    TaskContent,
    TypeContent,
    ValueContent,
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
    arguments: dict[str, StatementInstance]


@dataclass
class SymbolInstance(StatementInstance):
    @property
    def py_handle(self) -> typing.Any:
        raise NotImplementedError


@dataclass(repr=False)
class TypeInstance(TypeContent, SymbolInstance):
    py_type: typing.Any

    @property
    def py_handle(self) -> typing.Any:
        return self.py_type


@dataclass(repr=False)
class TaskInstance(TaskContent, SymbolInstance):
    subtasks: list[TaskInstance]
    expectations: list[ExpectationInstance | DatasetInstance | CodeInstance]


@dataclass(repr=False)
class ExpectationInstance(ExpectationContent, SymbolInstance):
    expectations: list[ExpectationInstance | DatasetInstance | CodeInstance]


@dataclass(repr=False)
class DatasetInstance(DatasetContent, SymbolInstance):
    records_batch: RecordBatch

    @property
    def py_handle(self) -> RecordBatch:
        return self.records_batch


@dataclass(repr=False)
class ValueInstance(ValueContent, SymbolInstance):
    @property
    def py_handle(self):
        return self.value


@dataclass(repr=False)
class ModelInstance(ModelContent, SymbolInstance):
    @property
    def py_handle(self):
        return self


@dataclass(repr=False)
class CodeInstance(CodeContent, SymbolInstance):
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
    stop: list[str] | None


@dataclass(slots=True)
class PromptSettings:
    model: ModelInstance
    temperature: float
    max_tokens: int
    stop: list[str] | None


@dataclass
class TextGeneration:
    text: str
    tokens: list[str]
    logits: list[float]
