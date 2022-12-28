from __future__ import annotations

import uuid
from collections import defaultdict
from dataclasses import dataclass, field
from typing import Any, Optional
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
class DerivedRoot:
    """The root for derived statement data. To simplify, we assume statements are never deleted and immutable."""

    # TODO @Architecture: how to represent statement derived state? when/where to (re)compute it?

    statements_by_parent: dict[UUID, list[Statement]] = field(
        default_factory=lambda: defaultdict(list)
    )


@dataclass(repr=False)
class File:
    path: str
    statements: list[Statement] = field(default_factory=list)
    id: UUID = field(default_factory=uuid.uuid4)

    def __str__(self):
        return self.path

    def __repr__(self):
        return f"<File {self.path}>"

    @property
    def root_statements(self) -> list[Statement]:
        return [statement for statement in self.statements if statement.parent is None]


UnresolvedStatement = tuple[str, str]


@dataclass(repr=False, frozen=True)
class Statement:
    file: File
    parent: Optional[Statement]
    index: int
    type: StatementType
    id: UUID = field(default_factory=uuid.uuid4)
    modifier: Optional[StatementModifier] = None
    name: Optional[str] = None
    text: Optional[str] = None
    value: Optional[dict] = None
    symbol_type: Optional[SymbolType] = None
    content: Optional[SymbolContent] = None
    reference: Optional[Statement | UnresolvedStatement] = None
    requirement: Optional[Requirement] = None
    compilation: Optional[Compilation] = None
    runconfig: Optional[RunConfiguration] = None

    _root: Optional[DerivedRoot] = None
    _source: Optional[Any] = None

    def __str__(self):
        path = self.file.path + ":" + str(self.absolute_index)
        if self.type == StatementType.DEFINITION:
            content_str = f"{self.content}"
        elif self.type in (StatementType.IMPORT, StatementType.REFERENCE):
            content_str = f"{self.reference}"
        elif self.type == StatementType.COMMENT:
            content_str = f"{len(self.text)}"
        else:
            raise ValueError(f"unknown statement type {self.type}")
        modifier_str = f" {self.modifier}" if self.modifier else ""
        return f"{path}{modifier_str} {self.type} {self.symbol_type} {self.name} {content_str}"

    def __repr__(self):
        return f"<Statement {self}>"

    @property
    def absolute_index(self) -> str:
        if self.parent:
            return f"{self.parent.absolute_index}.{self.index}"
        return str(self.index)

    @property
    def _root_(self) -> DerivedRoot:
        if self._root is None:
            raise RuntimeError("Statement._root is not set")
        return self._root

    def __post_init__(self):
        if self._root is None:
            return
        if self.parent is not None:
            self._root_.statements_by_parent[self.parent.id].append(self)

    @property
    def children(self):
        return self._root.statements_by_parent[self.id]

    @property
    def siblings(self):
        if self.parent is None:
            return self.file.root_statements
        return self.parent.children


@dataclass(repr=False)
class SymbolContent:
    type: SymbolType
    definition: Statement


@dataclass(repr=False)
class Task(SymbolContent):
    description: str

    def __str__(self):
        return f"({self.description})"


@dataclass(repr=False)
class Expectation(SymbolContent):
    description: str

    def __str__(self):
        return f"(description={self.description})"


@dataclass(repr=False)
class Dataset(SymbolContent):
    records: RecordBatch

    def __str__(self):
        return f"({len(self.records)}*{'<no schema>'})"


@dataclass(repr=False)
class Schema(SymbolContent):
    element: SchemaElement
    description: str = ""

    def __content_str__(self):
        return str(self.element)


@dataclass(repr=False)
class Value(SymbolContent):
    value: dict

    def __content_str__(self):
        return f"{self.value}"


@dataclass(repr=False)
class Model(SymbolContent):
    provider: str
    external_name: str
    settings: Optional[ModelInferenceSettings]
    default_settings: Optional[ModelInferenceSettings]

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
    code_text: Optional[str]
    code_function_name: Optional[str]
    builtin_id: Optional[str]

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
class Requirement:
    name: str
    version: str


@dataclass(repr=False)
class RunConfiguration:
    pass


@dataclass(repr=False)
class Compilation:
    id: UUID
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
