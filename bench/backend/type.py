from __future__ import annotations

import typing
import uuid
from dataclasses import dataclass

from bench.backend.provider import ModelHandle
from bench.language.schema import SchemaElement
from bench.language.type import Code, Dataset, Expectation, Model, Schema, Task, Value
from bench.utils.record import RecordBatch

AsyncCodeCallable = typing.Callable[..., typing.Coroutine]
SyncCodeCallable = typing.Callable[..., typing.Any]


@dataclass
class StatementInstance:
    instance_id: uuid.UUID
    children: list[StatementInstance]
    arguments: dict[str, StatementInstance]


@dataclass
class SymbolInstance(StatementInstance):
    @property
    def py_handle(self) -> typing.Any:
        raise NotImplementedError


@dataclass(repr=False)
class SchemaInstance(Schema, SymbolInstance):
    @property
    def py_handle(self) -> SchemaElement:
        return self.element


@dataclass(repr=False)
class TaskInstance(SymbolInstance, Task):
    schema: SchemaInstance


@dataclass(repr=False)
class ExpectationInstance(Expectation, SymbolInstance):
    pass


@dataclass(repr=False)
class DatasetInstance(Dataset, SymbolInstance):
    schema: SchemaInstance

    @property
    def py_handle(self) -> RecordBatch:
        return self.records


@dataclass(repr=False)
class ValueInstance(Value, SymbolInstance):
    schema: SchemaInstance

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
    schema: SchemaInstance
    code_callable: SyncCodeCallable | AsyncCodeCallable

    @property
    def py_handle(self) -> SyncCodeCallable | AsyncCodeCallable:
        return self.code_callable
