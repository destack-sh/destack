from __future__ import annotations

import uuid
from dataclasses import asdict, dataclass, field
from typing import Any, Literal, NamedTuple, Optional, Union
from uuid import UUID

from django.db import models

from bench.language.schema import SchemaElement, render_bsl
from bench.utils.record import RecordBatch, RecordList

LiteralValue = Union[dict[str, str], list["LiteralValue"], int, float, bool, str, None]


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
    REQUIREMENT = "require"
    COMPILATION = "compile"
    RUNCONFIG = "run"


@dataclass(repr=False)
class Module:
    name: str
    files: list[File]
    id: UUID = field(default_factory=uuid.uuid4)

    def __str__(self):
        return f"{self.name} ({len(self.files)} files)"

    def __repr__(self):
        return f"<Module {str(self)}>"


MOCK_MODULE = Module("<mock>", [])


@dataclass(repr=False)
class File:
    module: Module
    path: str
    statements: list[Statement] = field(default_factory=list)
    id: UUID = field(default_factory=uuid.uuid4)

    def __str__(self):
        return f"{self.module.name}/{self.path}"

    def __repr__(self):
        return f"<File {str(self)}>"

    @property
    def extension(self) -> str:
        if "." not in self.path:
            raise ValueError(f"{self} has no extension")
        return self.path.split(".")[-1]

    @property
    def path_without_extension(self) -> str:
        if "." in self.path:
            return self.path[: -len(self.extension) - 1]
        else:
            return self.path

    @property
    def root_statements(self) -> list[Statement]:
        return [statement for statement in self.statements if statement.parent is None]


MOCK_FILE = File(MOCK_MODULE, "<mock>")

StatementPath = NamedTuple("StatementPath", [("path", str), ("name", str)])


def statement_path_as_str(statement_path: StatementPath) -> str:
    return f"{statement_path.path}::{statement_path.name}"


def parse_statement_path(statement_path: str) -> StatementPath:
    if "::" not in statement_path:
        raise ValueError(f"invalid statement path: {statement_path}")
    path, name = statement_path.split("::")
    return StatementPath(path, name)


@dataclass(repr=False)
class Statement:
    file: File
    parent: Optional[Statement]
    index: int
    type: StatementType
    modifier: Optional[StatementModifier] = None
    name: Optional[str] = None
    text: Optional[str] = None
    symbol_type: Optional[SymbolType] = None
    content: Optional[SymbolContent] = None
    reference: Optional[Statement | StatementPath] = None
    id: UUID = field(default_factory=uuid.uuid4)

    _source: Optional[Any] = None

    def __str__(self):
        # TODO @Cleanup: Statement.__str__ looks suspiciously like a worse reconstruct.render_statement
        #  (also it's duplicated in models.Statement)
        path = self.file.path + ":" + str(self.absolute_index)
        if self.type == StatementType.DEFINITION:
            content_str = str(self.content)
        elif self.type in (StatementType.IMPORT, StatementType.REFERENCE):
            if isinstance(self.reference, Statement):
                content_str = f"{self.reference.file.path}::{self.reference.name}"
            else:
                content_str = statement_path_as_str(self.reference)
        elif self.type == StatementType.REDEFINITION:
            if isinstance(self.reference, Statement):
                content_str = f"{self.name} = {self.reference.file.path}::{self.reference.name}"
            else:
                content_str = f"{self.name} = {statement_path_as_str(self.reference)}"
        elif self.type == StatementType.COMMENT:
            content_str = str(len(self.text))
        elif self.type == StatementType.BLANK:
            content_str = ""
        else:
            raise ValueError(f"unknown statement type {self.type}")
        modifier_str = f" {self.modifier}" if self.modifier else ""
        return f"{path}{modifier_str} {self.type} {self.symbol_type} {self.name} {content_str}"

    def __repr__(self):
        return f"<Statement {self}>"

    @property
    def parent_id(self) -> Optional[UUID]:
        return self.parent.id if self.parent else None

    @property
    def reference_id(self) -> Optional[UUID]:
        return self.reference.id if isinstance(self.reference, Statement) else None

    @property
    def underlying_definition(self) -> Optional[Statement]:
        if self.type == StatementType.DEFINITION:
            return self
        elif isinstance(self.reference, Statement):
            return self.reference.underlying_definition
        else:
            return None

    @property
    def ungrouped(self) -> bool:
        """Whether this statement shouldn't be grouped with its siblings."""
        return self.defines_symbol and self.symbol_type not in (SymbolType.REQUIREMENT,)

    @property
    def referable(self) -> bool:
        return self.type in (
            StatementType.DEFINITION,
            StatementType.REDEFINITION,
            StatementType.IMPORT,
        )

    @property
    def defines_symbol(self) -> bool:
        return self.type in (StatementType.DEFINITION, StatementType.REDEFINITION)

    @property
    def is_parameter(self) -> bool:
        return self.modifier == StatementModifier.WITH and self.type == StatementType.REFERENCE

    @property
    def is_argument(self) -> bool:
        return self.modifier == StatementModifier.WITH and self.defines_symbol

    @property
    def is_alias(self):
        if isinstance(self.reference, StatementPath):
            return self.reference[1] != self.name
        elif isinstance(self.reference, Statement):
            return self.reference.name != self.name
        else:
            return False

    @property
    def requires_schema(self):
        return self.type == StatementType.DEFINITION and self.symbol_type in (
            SymbolType.TASK,
            SymbolType.CODE,
            SymbolType.DATASET,
        )

    @property
    def absolute_index(self) -> str:
        if self.parent:
            return f"{self.parent.absolute_index}.{self.index}"
        return str(self.index)


MOCK_STATEMENT = Statement(file=MOCK_FILE, parent=None, index=0, type=StatementType.DEFINITION)


@dataclass(repr=False)
class SymbolContent:
    definition: Statement

    @property
    def type(self) -> SymbolType:
        return self.definition.symbol_type


@dataclass(repr=False)
class Schema(SymbolContent):
    element: SchemaElement
    description: str = ""

    @property
    def bsl(self):
        return render_bsl(self.element)

    def __content_str__(self):
        return str(self.element)


@dataclass(repr=False)
class Task(SymbolContent):
    description: str

    def __str__(self):
        return f"({self.description})"


@dataclass(repr=False)
class Expectation(SymbolContent):
    description: str

    def __str__(self):
        return f"({self.description})"


@dataclass(repr=False)
class Dataset(SymbolContent):
    records: RecordBatch

    def __str__(self):
        return f"({len(self.records)})"


@dataclass(repr=False)
class Value(SymbolContent):
    value: LiteralValue

    def __content_str__(self):
        return f"{self.value}"


@dataclass(repr=False)
class Model(SymbolContent):
    provider: str
    external_name: str
    settings: Optional[ModelInferenceSettings]

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

    def as_dict(self, omit_empty: bool) -> dict[str, Any]:
        return asdict(
            self, dict_factory=lambda items: {k: v for k, v in items if not omit_empty or v}
        )


@dataclass(repr=False)
class Code(SymbolContent):
    language: Literal["python"]
    code: Optional[str]
    builtin_id: Optional[str]

    def __content_str__(self):
        # copied almost verbatim from Code.__str__
        if self.builtin_id:
            return f"builtin={self.builtin_id}"
        elif self.code:
            return f"length={len(self.code)}"
        else:
            raise ValueError(f"code has no content: {self}")

    @property
    def is_async(self):
        return "await " in self.code  # TODO @Cleanup: improve async functions detection


@dataclass(repr=False)
class Requirement(SymbolContent):
    name: Optional[str]
    version: Optional[str] = None

    def __str__(self):
        return f"{self.name}@{self.version}"


@dataclass(repr=False)
class Runconfig(SymbolContent):
    pass


@dataclass(repr=False)
class Compilation(SymbolContent):
    source_mappings: list["SourceMapping"] = field(default_factory=list)


@dataclass(repr=False)
class SourceMapping:
    source: Statement
    source_revision: int
    source_path: dict
    target: Statement
    target_revision: int
    target_path: dict


def get_default_symbol_content(definition: Statement, symbol_type: SymbolType) -> SymbolContent:
    if symbol_type == SymbolType.TASK:
        return Task(definition, description="")
    elif symbol_type == SymbolType.EXPECTATION:
        return Expectation(definition, description="")
    elif symbol_type == SymbolType.DATASET:
        return Dataset(definition, records=RecordList([]))
    elif symbol_type == SymbolType.VALUE:
        return Value(definition, value=None)
    elif symbol_type == SymbolType.CODE:
        return Code(definition, language="python", code="", builtin_id=None)
    elif symbol_type == SymbolType.REQUIREMENT:
        return Requirement(definition, name="", version="")
    elif symbol_type == SymbolType.RUNCONFIG:
        return Runconfig(definition)
    elif symbol_type == SymbolType.COMPILATION:
        return Compilation(definition)
