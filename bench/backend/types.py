from __future__ import annotations

import typing
import uuid
from dataclasses import dataclass
from functools import cached_property, partial

from django.db import models

from bench.backend.provider import ModelHandle
from bench.language.types import Code, Dataset, Model

AsyncCodeCallable = typing.Callable[..., typing.Coroutine]
SyncCodeCallable = typing.Callable[..., typing.Any]


class ProviderKey(models.TextChoices):
    OPENAI = "openai"
    GOOSEAI = "gooseai"
    AI21 = "ai21"


class StatementInstance:
    instance_id: uuid.UUID


@dataclass(repr=False)
class DatasetInstance(Dataset):
    pass


@dataclass(repr=False)
class ModelInstance(Model):
    handle: ModelHandle


@dataclass(repr=False)
class CodeInstance(Code):
    code_callable: SyncCodeCallable | AsyncCodeCallable

    @cached_property
    def callable_name(self) -> str:
        # get callable name (if partial get underlying func name)
        if isinstance(self.code_callable, partial):
            return self.code_callable.func.__name__
        else:
            return self.code_callable.__name__


SymbolInstance = typing.Union[DatasetInstance, ModelInstance, CodeInstance]
