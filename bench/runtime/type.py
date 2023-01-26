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
    TypeNode,
    Value,
)
from bench.runtime.bpl import DynamicPrompt
from bench.runtime.provider import ModelHandle
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
    @property
    def py_handle(self) -> TypeNode:
        return self.type_node


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
    handle: ModelHandle

    @property
    def py_handle(self) -> ModelHandle:
        return self.handle


@dataclass(repr=False)
class CodeInstance(Code, SymbolInstance):
    code_callable: SyncCodeCallable | AsyncCodeCallable
    prompt: typing.Optional[DynamicPrompt]

    @property
    def py_handle(self) -> SyncCodeCallable | AsyncCodeCallable:
        return self.code_callable
