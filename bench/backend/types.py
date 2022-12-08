from __future__ import annotations

import typing
from dataclasses import dataclass
from functools import cached_property, partial
from uuid import UUID

from bench.backend.provider import ModelHandle
from bench.models import ModelInferenceSettings, SymbolType
from bench.models.code import SymbolParameterType
from bench.utils.record import RecordBatch
from bench.utils.schema import SchemaElement

CodeCallable = typing.Callable[..., typing.Coroutine]
Value = typing.Any


@dataclass(repr=False)
class ResolvedSymbol:
    definition_id: UUID
    symbol_id: UUID
    name: str
    type: SymbolType

    def __str__(self):
        return f"{self.name}.{self.type}@{self.definition_id}({self.__content_str__()})"

    def __repr__(self):
        return f"<{self.__class__.__name__}: {str(self)}>"

    def __content_str__(self):
        return ""


@dataclass(repr=False)
class ResolvedDataset(ResolvedSymbol):
    schema: SchemaElement
    records: RecordBatch

    def __content_str__(self):
        return f"schema={self.schema}, length={len(self.records)}"


@dataclass(repr=False)
class LoadedDataset(ResolvedDataset):
    pass  # no additional attributes for now


@dataclass(repr=False)
class ResolvedModel(ResolvedSymbol):
    provider: str
    external_name: str
    settings: typing.Optional[ModelInferenceSettings]
    default_settings: typing.Optional[ModelInferenceSettings]

    def __content_str__(self):
        return f"provider={self.provider}/{self.external_name}"


@dataclass(repr=False)
class LoadedModel(ResolvedModel):
    handle: ModelHandle


@dataclass()
class ResolvedParameter:
    name: str
    type: SymbolParameterType


@dataclass(repr=False)
class ResolvedCode(ResolvedSymbol):
    input_schema: SchemaElement
    output_schema: SchemaElement
    code_text: typing.Optional[str]
    code_function_name: typing.Optional[str]
    builtin_id: typing.Optional[str]
    parameters: dict[str, ResolvedParameter]
    arguments: dict[str, ResolvedSymbol | typing.Any]

    def __content_str__(self):
        # copied almost verbatim from Code.__str__
        if self.builtin_id:
            content = f"builtin={self.builtin_id}"
        elif self.code_function_name and self.code_text:
            content = f"function={self.code_function_name},chars={len(self.code_text)},lines={len(self.code_text.splitlines())}"
        elif self.code_text:
            content = f"length={len(self.code_text)}"
        else:
            raise ValueError(f"code has no content: {self}")
        return f"{content},{self.input_schema}->{self.output_schema}"


@dataclass(repr=False)
class LoadedCode(ResolvedCode):
    code_callable: CodeCallable
    loaded_arguments: dict[str, ResolvedSymbol | typing.Any]

    @cached_property
    def callable_name(self) -> str:
        # get callable name (if partial get underlying func name)
        if isinstance(self.code_callable, partial):
            return self.code_callable.func.__name__
        else:
            return self.code_callable.__name__


LoadedSymbol = typing.Union[LoadedDataset, LoadedModel, LoadedCode]
