from __future__ import annotations

import typing
import uuid
from dataclasses import dataclass, field

from django.db import models

from bench.backend.provider import ModelHandle
from bench.language.schema import SchemaElement
from bench.language.type import Code, Dataset, Expectation, Model, Schema, Task, Value
from bench.utils.record import RecordBatch

AsyncCodeCallable = typing.Callable[..., typing.Coroutine]
SyncCodeCallable = typing.Callable[..., typing.Any]


class ProviderKey(models.TextChoices):
    OPENAI = "openai"
    GOOSEAI = "gooseai"
    AI21 = "ai21"


@dataclass
class StatementInstance:
    instance_id: uuid.UUID = field(default_factory=uuid.uuid4)


@dataclass
class SymbolInstance(StatementInstance):
    @property
    def py_handle(self) -> typing.Any:
        raise NotImplementedError


@dataclass(repr=False)
class SchemaInstance(SymbolInstance, Schema):
    @property
    def py_handle(self) -> SchemaElement:
        return self.element


@dataclass(repr=False)
class TaskInstance(SymbolInstance, Task):
    pass


@dataclass(repr=False)
class ExpectationInstance(SymbolInstance, Expectation):
    pass


@dataclass(repr=False)
class DatasetInstance(SymbolInstance, Dataset):
    @property
    def py_handle(self) -> RecordBatch:
        return self.records


@dataclass(repr=False)
class ValueInstance(SymbolInstance, Value):
    @property
    def py_handle(self):
        return self.value


@dataclass(repr=False)
class ModelInstance(SymbolInstance, Model):
    handle: ModelHandle | None = None

    @property
    def py_handle(self) -> ModelHandle:
        if self.handle is None:
            raise RuntimeError("handle is None")
        return self.handle


@dataclass(repr=False)
class CodeInstance(SymbolInstance, Code):
    code_callable: SyncCodeCallable | AsyncCodeCallable | None = None

    @property
    def py_handle(self) -> SyncCodeCallable | AsyncCodeCallable:
        if self.code_callable is None:
            raise RuntimeError("code_callable is None")
        return self.code_callable
