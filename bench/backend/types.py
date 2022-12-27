from __future__ import annotations

import typing
from dataclasses import dataclass
from functools import cached_property, partial
from uuid import UUID

from bench.backend.provider import ModelHandle
from bench.models import ModelInferenceSettings, StatementType, SymbolType
from bench.utils.record import RecordBatch
from bench.utils.schema import SchemaElement

CodeCallable = typing.Callable[..., typing.Coroutine]
Value = typing.Any


@dataclass(repr=False)
class StatementData:
    id: UUID
    file_id: UUID
    parent_id: typing.Optional[UUID]
    index: int
    name: str
    type: StatementType
    symbol_type: SymbolType
    children: list[UUID]
    parameters: dict[str, UUID]
    arguments: dict[str, UUID]
    content: typing.Optional[SymbolData]


@dataclass(repr=False)
class SymbolData:
    definition_id: UUID
    id: UUID
    type: SymbolType

    def __str__(self):
        return f"{self.type} {self.name}@{self.definition_id}({self.__content_str__()})"

    def __repr__(self):
        return f"<{self.__class__.__name__}: {str(self)}>"

    def __content_str__(self):
        return ""


@dataclass(repr=False)
class TaskData(SymbolData):
    input_schema: SchemaElement
    output_schema: SchemaElement
    description: str

    def __content_str__(self):
        return f"{self.input_schema}->({self.output_schema})"


@dataclass(repr=False)
class ExpectationData(SymbolData):
    description: str


@dataclass(repr=False)
class DatasetData(SymbolData):
    schema: SchemaElement
    records: RecordBatch

    def __content_str__(self):
        return f"schema={self.schema}, length={len(self.records)}"


@dataclass(repr=False)
class ValueData(SymbolData):
    value: dict

    def __content_str__(self):
        return f"{self.value}"


@dataclass(repr=False)
class ModelData(SymbolData):
    provider: str
    external_name: str
    settings: typing.Optional[ModelInferenceSettings]
    default_settings: typing.Optional[ModelInferenceSettings]

    def __content_str__(self):
        return f"provider={self.provider}/{self.external_name}"


@dataclass(repr=False)
class CodeData(SymbolData):
    input_schema: SchemaElement
    output_schema: SchemaElement
    code_text: typing.Optional[str]
    code_function_name: typing.Optional[str]
    builtin_id: typing.Optional[str]

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
class CompilationData:
    id: UUID
    definition_id: UUID
    source_id: UUID
    backend_models_ids: list[UUID]
    backend_models: list[StatementData]
    source_mappings: list["SourceMappingData"]


@dataclass(repr=False)
class SourceMappingData:
    id: UUID
    source_id: UUID
    source: StatementData
    source_revision: int
    source_path: dict
    target_id: UUID
    target: StatementData
    target_revision: int
    target_path: dict


# ====================
# Instantiated types for execution
# ====================


@dataclass(repr=False)
class LoadedDataset(DatasetData):
    pass  # no additional attributes for now


@dataclass(repr=False)
class LoadedModel(ModelData):
    handle: ModelHandle


@dataclass(repr=False)
class LoadedCode(CodeData):
    code_callable: CodeCallable
    loaded_arguments: dict[str, SymbolData | typing.Any]

    @cached_property
    def callable_name(self) -> str:
        # get callable name (if partial get underlying func name)
        if isinstance(self.code_callable, partial):
            return self.code_callable.func.__name__
        else:
            return self.code_callable.__name__


LoadedSymbol = typing.Union[LoadedDataset, LoadedModel, LoadedCode]
