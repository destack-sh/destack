from __future__ import annotations

import copy
import enum
import re
import typing
import uuid
from dataclasses import dataclass, field
from enum import Enum
from functools import cached_property
from typing import (
    Any,
    Generic,
    Literal,
    NamedTuple,
    Optional,
    OrderedDict,
    TypeVar,
    Union,
)
from uuid import UUID

from django.db import models

from bench.settings.utils import required_field


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
    EXTEND = "extend"
    LIKE = "like"
    UNLIKE = "unlike"
    CHECK = "check"


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


StatementPath = NamedTuple("StatementPath", [("path", str), ("name", str)])


def statement_path_as_str(statement_path: StatementPath) -> str:
    return f"{statement_path.path}:{statement_path.name}"


def parse_statement_path(statement_path: str) -> StatementPath:
    if ":" not in statement_path:
        raise ValueError(f"invalid statement path: {statement_path}")
    path, name = statement_path.split(":")
    return StatementPath(path, name)


SymbolContentT = TypeVar("SymbolContentT", bound="SymbolContent")


@dataclass(repr=False)
class Statement(Generic[SymbolContentT]):
    """A parsed but not interpreted statement in Bench source."""

    file: File
    parent: Optional[Statement]
    index: int
    type: StatementType
    modifier: Optional[StatementModifier] = None
    name: Optional[str] = None
    text: Optional[str] = None
    symbol_type: Optional[SymbolType] = None
    content: Optional[SymbolContentT] = None
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
    def infile_path(self) -> str:
        parent = self.parent
        ancestor_parts = [self.name]
        while parent is not None:
            ancestor_parts.append(parent.name)
            parent = parent.parent
        return ".".join(reversed(ancestor_parts))

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
        return self.is_parameter or self.type in (
            StatementType.DEFINITION,
            StatementType.REDEFINITION,
            StatementType.IMPORT,
        )

    @property
    def is_expect(self) -> bool:
        expectable_symbol = self.symbol_type in (
            SymbolType.TASK,
            SymbolType.CODE,
            SymbolType.DATASET,
        )
        has_expect_intent = self.modifier in (
            StatementModifier.LIKE,
            StatementModifier.UNLIKE,
            StatementModifier.CHECK,
        )
        return self.is_proper and (
            self.symbol_type == SymbolType.EXPECTATION or (expectable_symbol and has_expect_intent)
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
    def is_proper(self):
        return not self.is_argument and not self.is_parameter

    @property
    def is_extend(self):
        return self.modifier == StatementModifier.EXTEND

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


@dataclass(repr=False)
class InterpSymbol:
    """An interpreted - fully resolved, templated and validated - symbol from Bench source."""

    id: UUID = field(default_factory=uuid.uuid4)
    name: str = required_field()
    abstract: bool = field(default=False)
    modifier: Optional[StatementModifier] = None
    context: OrderedDict[str, "InterpSymbol"] = field(default_factory=OrderedDict)
    source: Optional[Statement] = None

    @property
    def symbol_type(self) -> SymbolType:
        return SYMBOL_TYPE_BY_CLASS[self.__class__]

    def __str__(self):
        modifier_str = f"{self.modifier} " if self.modifier else ""
        return f"{modifier_str}{self.symbol_type} {self.name} (source={self.source or '<unknown>'})"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}>"


class SymbolContent:
    def deepcopy(self) -> "SymbolContent":
        # default dataclass copy
        return self.__class__(**self.__dict__)  # type: ignore


LiteralValue = Union[dict[str, str], list["LiteralValue"], int, float, bool, str, None]
PRIMITIVE_TYPES = [TypeTag.NULL, TypeTag.BOOLEAN, TypeTag.NUMBER, TypeTag.STRING]


@dataclass
class TypeNode(SymbolContent):
    id: UUID = field(default_factory=uuid.uuid4)
    name: Optional[str] = required_field()
    tag: TypeTag = required_field()
    description: Optional[str] = None
    value: Optional[LiteralValue] = None  # for literal types
    reference: Union[None, str, "TypeNode", "Type"] = None
    # source reference is separate as the resolved TypeNode may not contain the name
    source_reference: Optional[str] = None
    children: Optional[list["TypeNode"]] = None

    def __str__(self):
        name_str = f"{self.name} " if self.name else ""
        return f"{name_str}{self.tag.value}"

    def deepcopy(self) -> "TypeNode":
        return TypeNode(
            name=self.name,
            tag=self.tag,
            description=self.description,
            # revert to reference by name for copy (to avoid carrying the whole tree)
            reference=self.source_reference,
            value=self.value,
            children=[child.deepcopy() for child in self.children] if self.children else None,
        )

    def walk(self, path: list[TypeNode] | None = None):
        if path is None:
            path = [self]
        else:
            path = path + [self]
        yield self
        if self.children:
            for child in self.children:
                if child in path:
                    continue  # break cycles (allowed, but we don't want to traverse them)
                yield from child.walk(path)

    def to_type(self) -> "Type":
        if self.name is None:
            raise ValueError("cannot convert anonymous type to Type")
        return Type(**self.deepcopy().__dict__)

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

    @property
    def is_union_with_none(self) -> bool:
        return self.tag == TypeTag.UNION and any(
            child.tag == TypeTag.NULL for child in self.children
        )

    @property
    def is_flat(self) -> bool:
        """Whether this type can be represented as a single un-nested primitive value."""
        if self.tag in PRIMITIVE_TYPES:
            return True
        elif self.tag == TypeTag.ENUM:
            return self.head_type.is_flat
        elif self.tag == TypeTag.UNION:
            return all(child.is_flat for child in self.children)
        else:
            return False

    def child(self, key: str) -> TypeNode:
        if self.children is None:
            raise ValueError(f"find cannot be used on {self}")
        for node in self.children:
            if node.name == key:
                return node
        raise KeyError(f"key {key} not found in {self}")


@dataclass(repr=False)
class Type(InterpSymbol, TypeNode):
    expectations: list[Expectation | Task | Dataset | Code] = field(default_factory=list)

    # override __str__/__repr__ to preserve InterpSymbol's __str__/__repr__
    def __str__(self):
        return InterpSymbol.__str__(self)

    def __repr__(self):
        return InterpSymbol.__repr__(self)


@dataclass(repr=False)
class CapabilityContent(SymbolContent):
    description: str

    def __str__(self):
        return f"({self.description})"


@dataclass(repr=False)
class Capability(InterpSymbol, CapabilityContent):
    expectations: list[Expectation | Task | Dataset | Code] = field(default_factory=list)
    tasks: list[Task] = field(default_factory=list)
    capabilities: list[Capability] = field(default_factory=list)


@dataclass(repr=False)
class TaskContent(SymbolContent):
    type_node: TypeNode
    description: str


@dataclass(repr=False)
class Task(InterpSymbol, TaskContent):
    type: Type = required_field()
    implementation: Optional[Code] = field(default=None)
    expectations: list[Expectation | Task | Dataset | Code] = field(default_factory=list)
    steps: list[Task | Code] = field(default_factory=list)


@dataclass(repr=False)
class ExpectationContent(SymbolContent):
    description: str
    on: Optional[str]

    def __str__(self):
        on_str = f" on {self.on}" if self.on else ""
        return f"({self.description}{on_str})"


@dataclass(repr=False)
class Expectation(InterpSymbol, ExpectationContent):
    expectations: list[Expectation | Task | Dataset | Code] = field(default_factory=list)


@dataclass(repr=False)
class DatasetContent(SymbolContent):
    language: Literal["csv"] | Literal["json"] | Literal["jsonl"]
    records: list[dict[str, LiteralValue]]
    type_node: TypeNode
    description: Optional[str]

    def deepcopy(self) -> "DatasetContent":
        return DatasetContent(
            language=self.language,
            records=copy.deepcopy(self.records),
            type_node=self.type_node,
            description=self.description,
        )

    def __str__(self):
        return f"({len(self.records)})"


@dataclass(repr=False)
class Dataset(InterpSymbol, DatasetContent):
    type: Type = required_field()


@dataclass(repr=False)
class ValueContent(SymbolContent):
    value: LiteralValue
    description: Optional[str]

    def __content_str__(self):
        return f"{self.value}"


@dataclass(repr=False)
class Value(InterpSymbol, ValueContent):
    pass


@dataclass(repr=False)
class ModelContent(SymbolContent):
    provider: str
    external_name: str

    def __str__(self):
        return f"provider={self.provider}/{self.external_name}"


@dataclass(repr=False)
class Model(InterpSymbol, ModelContent):
    pass


@dataclass(repr=False)
class CodeContent(SymbolContent):
    description: Optional[str]
    language: Literal["python"] | Literal["bpl"]
    code: Optional[str]
    builtin_id: Optional[str]
    type_node: TypeNode

    def __str__(self):
        # copied almost verbatim from Code.__str__
        if self.builtin_id:
            return f"builtin={self.builtin_id}"
        elif self.code:
            return f"code={len(self.code)}"


@dataclass(repr=False)
class Code(InterpSymbol, CodeContent):
    type: Type = required_field()


@dataclass(repr=False)
class RequirementContent(SymbolContent):
    module_name: Optional[str]
    version: Optional[str]

    def __str__(self):
        return f"{self.module_name}@{self.version}"


@dataclass(repr=False)
class Requirement(InterpSymbol, RequirementContent):
    pass


@dataclass(repr=False)
class RunconfigContent(SymbolContent):
    def __str__(self):
        return ""


@dataclass(repr=False)
class Runconfig(InterpSymbol, RunconfigContent):
    codes: list[Code] = field(default_factory=list)
    tasks: list[Task] = field(default_factory=list)
    compilations: list[Compilation] = field(default_factory=list)


@dataclass(repr=False)
class CompilationContent(SymbolContent):
    source_mappings: list["SourceMapping"]

    def __str__(self):
        return ""


@dataclass(repr=False)
class Compilation(InterpSymbol, CompilationContent):
    tasks: list[Task] = field(default_factory=list)
    models: list[Model] = field(default_factory=list)


@dataclass(repr=False)
class SourceMapping:
    source_id: UUID
    source_revision: int
    source_path: dict
    target_id: UUID
    target_revision: int
    target_path: dict


SYMBOL_CLASS_BY_TYPE: dict[SymbolType, typing.Type[InterpSymbol]] = {
    SymbolType.TYPE: Type,
    SymbolType.CAPABILITY: Capability,
    SymbolType.TASK: Task,
    SymbolType.EXPECTATION: Expectation,
    SymbolType.DATASET: Dataset,
    SymbolType.VALUE: Value,
    SymbolType.MODEL: Model,
    SymbolType.CODE: Code,
    SymbolType.REQUIREMENT: Requirement,
    SymbolType.RUNCONFIG: Runconfig,
    SymbolType.COMPILATION: Compilation,
}
SYMBOL_TYPE_BY_CLASS: dict[typing.Type[InterpSymbol], SymbolType] = {
    v: k for k, v in SYMBOL_CLASS_BY_TYPE.items()
}


EMPTY_FUNC_TYPE = TypeNode(
    name=None,
    tag=TypeTag.FUNCTION,
    children=[
        TypeNode(name="input", tag=TypeTag.STRUCT, children=[]),
        TypeNode(name="output", tag=TypeTag.NULL),
    ],
)
EMPTY_STRUCT_TYPE = TypeNode(name=None, tag=TypeTag.STRUCT, children=[])
