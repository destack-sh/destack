from __future__ import annotations

import enum
import re
import typing
import uuid
from dataclasses import dataclass, field
from enum import Enum
from functools import cached_property
from typing import Any, Literal, Optional, Union
from uuid import UUID

from django.db import models


@dataclass(repr=False)
class SourceFile:
    path: str
    content: str

    def __str__(self):
        return f"{self.path} ({len(self.linebreaks)} lines, {len(self.content)} characters)"

    def __repr__(self):
        # truncate content on both sides
        max_length = 250
        if len(self.content) > max_length:
            prefix_content = self.content[: max_length // 2]
            postfix_content = self.content[-max_length // 2 :]
            lines_omitted = (
                len(self.linebreaks)
                - len(prefix_content.splitlines())
                - len(postfix_content.splitlines())
            )
            content = (
                f"{prefix_content}\n... ({lines_omitted} lines omitted) ...\n{postfix_content}"
            )
        else:
            content = self.content
        return f"<{self.__class__.__name__}: {str(self)}\n{content}\n>"

    @cached_property
    def linebreaks(self) -> list[int]:
        # index start of each line
        linebreaks = [0]
        for match in re.finditer(r"\r?\n", self.content):
            linebreaks.append(match.end())
        return linebreaks

    def line(self, line_number: int) -> str:
        index = line_number - 1
        if index >= len(self.linebreaks):
            raise IndexError(f"line {index} does not exist")
        elif index == len(self.linebreaks) - 1:
            return self.content[self.linebreaks[index] :]
        else:
            return self.content[self.linebreaks[index] : self.linebreaks[index + 1]]


class TokenType(enum.Enum):
    NEWFILE = "newfile"
    INDENT = "indent"
    NEWLINE = "newline"
    COMMENT = "comment"
    KEYWORD = "keyword"
    SEPARATOR = "separator"
    IDENTIFIER = "identifier"
    LITERAL = "literal"
    DESCRIPTION = "description"
    MARK_OPTIONAL = "mark_optional"
    BRACKET = "bracket"


@dataclass
class Token:
    source_file: SourceFile
    line_number: int
    line_span: int
    start_column: int
    end_column: int
    type: TokenType
    value: Union[None, str, enum.Enum]
    value_extras: Optional[dict[str, str]]

    def __str__(self):
        if self.value_extras:
            extras_str = ", ".join(f"{key}={value}" for key, value in self.value_extras.items())
            extras_str = f" ({extras_str})"
        else:
            extras_str = ""
        return f"{self.type.value} {self.value_truncated}{extras_str} ({self.source_file.path} {self.location_in_file})"

    @cached_property
    def value_truncated(self) -> str:
        # truncate value if too long
        max_length = 100
        if self.value is None:
            value = "<none>"
        elif isinstance(self.value, str):
            if len(self.value) > max_length:
                value = f"{self.value[: max_length // 2]}...{self.value[-max_length // 2:]}"
            else:
                value = self.value
        elif isinstance(self.value, enum.Enum):
            value = self.value.value
        else:
            raise TypeError(f"unexpected value type {type(self.value)}")
        # replace newlines with literal \n
        value = value.replace("\n", "\\n")
        return value

    @cached_property
    def location_in_file(self):
        if self.line_span > 1:
            loc = f"{self.line_number}:{self.start_column}-{self.line_number + self.line_span}:{self.end_column}"
        else:
            loc = f"{self.line_number}:{self.start_column}-{self.end_column}"
        return loc


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
    COMMENT = "comment"  # #
    BLANK = "blank"  # ...


class StatementModifier(models.TextChoices):
    """A modifier to a Bench statement."""

    VAR = "var"
    WITH = "with"
    LIKE = "like"
    UNLIKE = "unlike"
    VERIFY = "verify"


class SymbolType(models.TextChoices):
    """The type of symbol content."""

    TYPE = "type"
    CAPABILITY = "capability"
    TASK = "task"
    EXPECTATION = "expect"
    CODE = "code"
    MODEL = "model"
    DATASET = "data"
    VALUE = "value"
    REQUIREMENT = "require"
    COMPILATION = "compile"
    RUNCONFIG = "run"


class TypeTag(Enum):
    """The type of type node."""

    STRING = "string"
    NUMBER = "number"
    BOOLEAN = "boolean"
    ARRAY = "array"
    TUPLE = "tuple"
    MAP = "map"
    STRUCT = "struct"
    FUNCTION = "function"
    UNION = "union"
    INTERSECTION = "intersection"
    ENUM = "enum"
    LITERAL = "literal"
    NULL = "null"
    ANY = "any"
    TYPE_REFERENCE = "ref"


@dataclass(repr=False)
class Module:
    name: str
    files: list[File] = field(default_factory=list)
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

StatementPath = typing.NamedTuple("StatementPath", [("path", str), ("name", str)])


def statement_path_as_str(statement_path: StatementPath) -> str:
    return f"{statement_path.path}:{statement_path.name}"


def parse_statement_path(statement_path: str) -> StatementPath:
    if ":" not in statement_path:
        raise ValueError(f"invalid statement path: {statement_path}")
    path, name = statement_path.split(":")
    return StatementPath(path, name)


@dataclass(repr=False)
class Statement:
    """A parsed but not interpreted statement in Bench source."""

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
        from bench.language.reconstruct import render_statement

        loc = self.file.path + ":" + str(self.absolute_index)
        try:
            content = render_statement(self, include_content=False)
        except ValueError:
            content = "<invalid>"
        return f"{loc} {content}"

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
        return self.modifier == StatementModifier.VAR

    @property
    def is_argument(self) -> bool:
        return self.modifier == StatementModifier.WITH

    @property
    def is_alias(self):
        if isinstance(self.reference, StatementPath):
            return self.reference[1] != self.name
        elif isinstance(self.reference, Statement):
            return self.reference.name != self.name
        else:
            return False

    @property
    def absolute_index(self) -> str:
        if self.parent:
            return f"{self.parent.absolute_index}.{self.index}"
        return str(self.index)


SymbolContentT = typing.TypeVar("SymbolContentT", bound="SymbolContent")


@dataclass(repr=False)
class InterpStatement(typing.Generic[SymbolContentT]):
    """An interpreted - fully resolved, imputed and validated - statement from Bench source."""

    source: Statement

    @property
    def content(self) -> SymbolContentT:
        return self.source.content

    @property
    def modifier(self):
        return self.source.modifier

    @property
    def name(self):
        return self.source.name

    @property
    def symbol_type(self):
        return self.source.symbol_type


MOCK_STATEMENT = Statement(file=MOCK_FILE, parent=None, index=0, type=StatementType.DEFINITION)


@dataclass(repr=False)
class SymbolContent:
    definition: Statement


LiteralValue = Union[dict[str, str], list["LiteralValue"], int, float, bool, str, None]
PRIMITIVE_TYPES = [TypeTag.NULL, TypeTag.BOOLEAN, TypeTag.NUMBER, TypeTag.STRING]


@dataclass
class TypeNode:
    name: Optional[str]
    type: TypeTag
    required: bool = True
    description: Optional[str] = None
    reference: Union[None, str, "TypeNode"] = None
    value: Optional[LiteralValue] = None  # for literal types
    # source reference is separate as the resolved TypeNode may not contain the name
    source_reference: Optional[str] = None
    children: Optional[list["TypeNode"]] = None

    def __str__(self):
        return f"{self.name or '<anon>'}: {self.type}"

    @property
    def keys(self) -> list[str]:
        if self.children is None:
            return []
        else:
            return [e.name for e in self.children if e.name is not None]

    @property
    def input(self) -> TypeNode:  # for function types
        return self.child("input")

    @property
    def output(self) -> TypeNode:  # for function types
        return self.child("output")

    @property
    def head_type(self) -> TypeNode:  # for enum types
        return self.children[0]

    @property
    def members(self) -> list[TypeNode]:  # for enum types
        return self.children[1:]

    def child(self, key: str) -> TypeNode:
        if self.children is None:
            raise ValueError(f"find cannot be used on {self}")
        for node in self.children:
            if node.name == key:
                return node
        raise KeyError(f"key {key} not found in {self}")


@dataclass(repr=False)
class TypeContent(SymbolContent):
    type_node: TypeNode
    description: Optional[str]

    def __str__(self):
        return str(self.type_node)


@dataclass(repr=False)
class Type(InterpStatement[TypeContent]):
    @property
    def node(self) -> TypeNode:
        return self.content.type_node


@dataclass(repr=False)
class CapabilityContent(SymbolContent):
    description: str

    def __str__(self):
        return f"({self.description})"


@dataclass(repr=False)
class Capability(InterpStatement[CapabilityContent]):
    expectations: list[Expectation]
    tasks: list[Task]
    capabilities: list[Capability]


@dataclass(repr=False)
class TaskContent(SymbolContent):
    type_node: TypeNode


@dataclass(repr=False)
class Task(InterpStatement[TaskContent]):
    expectations: list[Expectation]
    items: list[Task | Code]


@dataclass(repr=False)
class ExpectationContent(SymbolContent):
    description: str
    on: Optional[str]

    def __str__(self):
        on_str = f" on {self.on}" if self.on else ""
        return f"({self.description}{on_str})"


@dataclass(repr=False)
class Expectation(InterpStatement[ExpectationContent]):
    expectations: list[Expectation | Task | Dataset | Code]


@dataclass(repr=False)
class DatasetContent(SymbolContent):
    language: Literal["csv"] | Literal["json"] | Literal["jsonl"]
    records: list[dict[str, LiteralValue]]
    type_node: TypeNode
    description: Optional[str]

    def __str__(self):
        return f"({len(self.records)})"


@dataclass(repr=False)
class Dataset(InterpStatement[DatasetContent]):
    pass


@dataclass(repr=False)
class ValueContent(SymbolContent):
    value: LiteralValue
    description: Optional[str]

    def __content_str__(self):
        return f"{self.value}"


@dataclass(repr=False)
class Value(InterpStatement[ValueContent]):
    pass


@dataclass(repr=False)
class ModelContent(SymbolContent):
    provider: str
    external_name: str

    def __content_str__(self):
        return f"provider={self.provider}/{self.external_name}"


@dataclass(repr=False)
class Model(InterpStatement[ModelContent]):
    pass


@dataclass(repr=False)
class CodeContent(SymbolContent):
    description: Optional[str]
    language: Literal["python"] | Literal["bpl"]
    code: Optional[str]
    builtin_id: Optional[str]
    type_node: TypeNode

    def __content_str__(self):
        # copied almost verbatim from Code.__str__
        if self.builtin_id:
            return f"builtin={self.builtin_id}"
        elif self.code:
            return f"length={len(self.code)}"
        else:
            raise ValueError(f"code has no content: {self}")


@dataclass(repr=False)
class Code(InterpStatement[CodeContent]):
    pass


@dataclass(repr=False)
class RequirementContent(SymbolContent):
    name: Optional[str]
    version: Optional[str] = None

    def __str__(self):
        return f"{self.name}@{self.version}"


@dataclass(repr=False)
class Requirement(InterpStatement[RequirementContent]):
    pass


@dataclass(repr=False)
class RunconfigContent(SymbolContent):
    def __str__(self):
        return ""


@dataclass(repr=False)
class Runconfig(InterpStatement[RunconfigContent]):
    pass


@dataclass(repr=False)
class CompilationContent(SymbolContent):
    source_mappings: list["SourceMapping"] = field(default_factory=list)

    def __str__(self):
        return ""


@dataclass(repr=False)
class Compilation(InterpStatement[CompilationContent]):
    tasks: list[Task]
    models: list[Model]


@dataclass(repr=False)
class SourceMapping:
    source_id: UUID
    source_revision: int
    source_path: dict
    target_id: UUID
    target_revision: int
    target_path: dict


EMPTY_FUNC_TYPE = TypeNode(
    name=None,
    type=TypeTag.FUNCTION,
    children=[
        TypeNode("input", TypeTag.STRUCT, children=[]),
        TypeNode("output", TypeTag.NULL),
    ],
)
EMPTY_STRUCT_TYPE = TypeNode(None, TypeTag.STRUCT, children=[])
