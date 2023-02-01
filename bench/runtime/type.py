from __future__ import annotations

import typing
from dataclasses import dataclass

from bench.language.parse import ModuleIndex
from bench.language.type import Code, Dataset, Model, Module, Type, Value
from bench.settings.utils import required_field
from bench.utils.record import RecordBatch

AsyncCodeCallable = typing.Callable[..., typing.Coroutine]
SyncCodeCallable = typing.Callable[..., typing.Any]


@dataclass(repr=False)
class ModuleInstance:
    module: Module
    index: ModuleIndex


@dataclass
class SymbolInstance:
    @property
    def py_handle(self) -> typing.Any:
        raise NotImplementedError


@dataclass(repr=False)
class TypeInstance(SymbolInstance, Type):
    py_type: typing.Any = required_field()

    @property
    def py_handle(self) -> typing.Any:
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
    prompt: typing.Optional[DynamicPrompt] = required_field()

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
