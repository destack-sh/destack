from __future__ import annotations

import enum
import typing
from dataclasses import dataclass, field
from uuid import UUID

from django.db import models

from bench.utils.record import RecordBatch
from bench.utils.schema import SchemaElement


# TODO @Cleanup: don't use Django's TextChoices inside language
#  (it carries all the Django baggage into all language-dependent code like workers)
#  Can probably use a custom enum.Enum subclass instead (or monkey-patch somehow)
class StatementType(models.TextChoices):
    """The type of Bench statement."""

    # symbol statements
    IMPORT = "import"  # import
    DEFINITION = "def"  # :
    REFERENCE = "ref"  #
    REDEFINITION = "redef"  # =
    # non-symbol statements
    REQUIREMENT = "requirement"  # require
    COMPILATION = "compilation"  # compile
    RUNCONFIG = "run"  # run
    COMMENT = "comment"  # //
    BLANK = "blank"  # used while creating a new statement


class StatementModifier(models.TextChoices):
    """A modifier to a Bench statement."""

    WITH = "with"
    LIKE = "like"
    UNLIKE = "unlike"
    VERIFY = "verify"


class SymbolType(models.TextChoices):
    """The type of symbol content."""

    SCHEMA = "schema"
    TASK = "task"
    EXPECTATION = "expect"
    CODE = "code"
    MODEL = "model"
    DATASET = "data"
    VALUE = "value"


@dataclass(repr=False)
class File:
    id: UUID
    statements: list[Statement] = field(default_factory=list)


@dataclass(repr=False)
class Statement:
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
    content: typing.Optional[SymbolContent]


@dataclass(repr=False)
class SymbolContent:
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
class Task(SymbolContent):
    input_schema: SchemaElement
    output_schema: SchemaElement
    description: str

    def __content_str__(self):
        return f"{self.input_schema}->({self.output_schema})"


@dataclass(repr=False)
class Expectation(SymbolContent):
    description: str


@dataclass(repr=False)
class Dataset(SymbolContent):
    schema: SchemaElement
    records: RecordBatch

    def __content_str__(self):
        return f"schema={self.schema}, length={len(self.records)}"


@dataclass(repr=False)
class Value(SymbolContent):
    value: dict

    def __content_str__(self):
        return f"{self.value}"


@dataclass(repr=False)
class Model(SymbolContent):
    provider: str
    external_name: str
    settings: typing.Optional[ModelInferenceSettings]
    default_settings: typing.Optional[ModelInferenceSettings]

    def __content_str__(self):
        return f"provider={self.provider}/{self.external_name}"


@dataclass(repr=False)
class ModelInferenceSettings:
    max_tokens: int = 256
    temperature: float = 0.7
    top_p: float = 1.0
    n: int = 1
    logprobs: int = 2
    stop: list[str] = field(default_factory=list)


@dataclass(repr=False)
class Code(SymbolContent):
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
class Compilation:
    id: UUID
    definition_id: UUID
    source_id: UUID
    backend_models_ids: list[UUID]
    backend_models: list[Statement]
    source_mappings: list["SourceMapping"]


@dataclass(repr=False)
class SourceMapping:
    id: UUID
    source_id: UUID
    source: Statement
    source_revision: int
    source_path: dict
    target_id: UUID
    target: Statement
    target_revision: int
    target_path: dict
