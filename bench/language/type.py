from __future__ import annotations

import enum
import re
import uuid
from dataclasses import asdict, dataclass, field
from enum import Enum
from functools import cached_property
from typing import Any, Literal, NamedTuple, Optional, Union
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

    def line(self, index: int) -> str:
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
    NULL = "null"
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

StatementPath = NamedTuple("StatementPath", [("path", str), ("name", str)])


def statement_path_as_str(statement_path: StatementPath) -> str:
    return f"{statement_path.path}:{statement_path.name}"


def parse_statement_path(statement_path: str) -> StatementPath:
    if ":" not in statement_path:
        raise ValueError(f"invalid statement path: {statement_path}")
    path, name = statement_path.split(":")
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


MOCK_STATEMENT = Statement(file=MOCK_FILE, parent=None, index=0, type=StatementType.DEFINITION)


@dataclass(repr=False)
class SymbolContent:
    definition: Statement


@dataclass(repr=False)
class Type(SymbolContent):
    type_node: TypeNode
    description: Optional[str]

    def __str__(self):
        return str(self.type_node)


LiteralValue = Union[dict[str, str], list["LiteralValue"], int, float, bool, str, None]
PRIMITIVE_TYPES = [TypeTag.NULL, TypeTag.BOOLEAN, TypeTag.NUMBER, TypeTag.STRING]


@dataclass
class TypeNode:
    name: Optional[str]
    type: TypeTag
    required: bool = True
    description: Optional[str] = None
    reference: Union[None, str, "TypeNode"] = None
    # source reference is separate as the resolved TypeNode may not contain the name
    source_reference: Optional[str] = None
    children: Optional[list["TypeNode"]] = None

    def __str__(self):
        return f"{self.name}: {self.type}"

    @property
    def keys(self) -> list[str]:
        if self.children is None:
            return []
        else:
            return [e.name for e in self.children]

    @property
    def input(self) -> TypeNode:
        return self.child("input")

    @property
    def output(self) -> TypeNode:
        return self.child("output")

    def child(self, key: str) -> TypeNode:
        if self.children is None:
            raise ValueError(f"find cannot be used on {self}")
        for node in self.children:
            if node.name == key:
                return node
        raise KeyError(f"key {key} not found in {self}")


@dataclass(repr=False)
class Capability(SymbolContent):
    description: str

    def __str__(self):
        return f"({self.description})"


@dataclass(repr=False)
class Task(SymbolContent):
    description: str
    type_node: TypeNode

    def __str__(self):
        return f"({self.description})"


@dataclass(repr=False)
class Expectation(SymbolContent):
    description: str

    def __str__(self):
        return f"({self.description})"


@dataclass(repr=False)
class Dataset(SymbolContent):
    language: Literal["csv"] | Literal["json"] | Literal["jsonl"]
    records: list[dict[str, LiteralValue]]
    type_node: TypeNode
    description: Optional[str]

    def __str__(self):
        return f"({len(self.records)})"


@dataclass(repr=False)
class Value(SymbolContent):
    value: LiteralValue
    description: Optional[str]

    def __content_str__(self):
        return f"{self.value}"


@dataclass(repr=False)
class Model(SymbolContent):
    provider: str
    external_name: str
    settings: Optional[ModelInferenceSettings]

    def __content_str__(self):
        return f"provider={self.provider}/{self.external_name}"


# TODO @Cleanup: ModelInferenceSettings should probably be just a built-in Type.
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
    description: Optional[str]
    language: Literal["python"]
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
    def __str__(self):
        return ""


@dataclass(repr=False)
class Compilation(SymbolContent):
    source_mappings: list["SourceMapping"] = field(default_factory=list)

    def __str__(self):
        return ""


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


def get_default_symbol_content(definition: Statement, symbol_type: SymbolType) -> SymbolContent:
    if symbol_type == SymbolType.TYPE:
        return Type(
            definition, description="", type_node=TypeNode(None, TypeTag.STRUCT, children=[])
        )
    elif symbol_type == SymbolType.TASK:
        return Task(definition, description="", type_node=EMPTY_FUNC_TYPE)
    elif symbol_type == SymbolType.CAPABILITY:
        return Capability(definition, description="")
    elif symbol_type == SymbolType.EXPECTATION:
        return Expectation(definition, description="")
    elif symbol_type == SymbolType.DATASET:
        return Dataset(
            definition,
            description="",
            language="jsonl",
            records=[],
            type_node=EMPTY_STRUCT_TYPE,
        )
    elif symbol_type == SymbolType.VALUE:
        return Value(definition, value=None, description="")
    elif symbol_type == SymbolType.CODE:
        return Code(
            definition,
            description="",
            language="python",
            code="",
            builtin_id=None,
            type_node=EMPTY_FUNC_TYPE,
        )
    elif symbol_type == SymbolType.REQUIREMENT:
        return Requirement(definition, name="", version="")
    elif symbol_type == SymbolType.RUNCONFIG:
        return Runconfig(definition)
    elif symbol_type == SymbolType.COMPILATION:
        return Compilation(definition)
