from __future__ import annotations

import abc
import copy
import enum
import re
import typing
import uuid
from dataclasses import dataclass, field
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

import PIL.Image
import pydub
from django.db import models

from bench.utils.fractional import INTEGER_ZERO
from bench.utils.func import dict_minus
from bench.utils.utils import required_field


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
    INCLUDE = "include"
    LIKE = "like"
    UNLIKE = "unlike"
    CHECK = "check"
    MAGIC = "magic"
    LOCAL = "local"


class SymbolType(models.TextChoices):
    """The type of symbol content."""

    # TODO @Language: merge value into data
    #  Simply typed version could be root is_array flag in addition to root_type_tag

    TYPE = "type"
    CAPABILITY = "capability"
    TASK = "task"
    EXPECTATION = "expect"
    CODE = "code"
    MODEL = "model"
    PROGRAM = "program"
    DATA = "data"
    REQUIREMENT = "require"
    BUILD = "build"
    EVALUATE = "evaluate"
    RUNCONFIG = "run"
    BLOCK = "block"


class TypeTag(models.TextChoices):
    """The type of type node."""

    STRING = "string"
    NUMBER = "number"
    BOOLEAN = "boolean"
    EMBEDDING = "embedding"
    IMAGE = "image"
    VIDEO = "video"
    AUDIO = "audio"
    FILE = "file"
    ARRAY = "array"
    STRUCT = "struct"
    JSON = "json"
    FUNCTION = "function"
    UNION = "union"
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
    generated: bool = False

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
    order_key: str
    type: StatementType
    modifier: Optional[StatementModifier] = None
    name: Optional[str] = None
    text: Optional[str] = None
    symbol_type: Optional[SymbolType] = None
    content: Optional[SymbolContentT] = None
    reference: Optional[Statement | StatementPath | UUID] = None
    id: UUID = field(default_factory=uuid.uuid4)
    generated: bool = False

    _source: Optional[Any] = None

    def __str__(self):
        from bench.language.reconstruct import render_statement

        loc = self.file.path + ":" + str(self.infile_path)
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
        ancestor_parts = [self.name or "<anon>"]
        seen_ids = {self.id}
        while parent is not None:
            if parent.id in seen_ids:
                # :CircularAncestry
                # circuit breaker: ignore here because this is an error in indexing
                ancestor_parts.append("<!loop>")
                break
            ancestor_parts.append(parent.name or "<anon>")
            seen_ids.add(parent.id)
            parent = parent.parent
        return ".".join(reversed(ancestor_parts))

    @property
    def fqn(self) -> str:
        return f"{self.file.module.name}.{self.file.path.replace('/', '.')}.{self.name}"

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
    def has_reference(self) -> bool:
        return not self.is_parameter and self.type in (
            StatementType.REDEFINITION,
            StatementType.IMPORT,
            StatementType.REFERENCE,
        )

    @property
    def is_expectable_symbol(self) -> bool:
        return self.symbol_type in (SymbolType.TASK, SymbolType.CODE, SymbolType.DATA)

    @property
    def is_expect(self) -> bool:
        has_expect_intent = self.modifier in (
            StatementModifier.LIKE,
            StatementModifier.UNLIKE,
            StatementModifier.CHECK,
        )
        return self.is_real and (
            self.symbol_type == SymbolType.EXPECTATION
            or (self.is_expectable_symbol and has_expect_intent)
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
    def is_real(self):
        return not self.is_argument and not self.is_parameter

    @property
    def is_extend(self):
        return self.modifier == StatementModifier.INCLUDE

    @property
    def is_alias(self):
        if isinstance(self.reference, StatementPath):
            return self.reference[1] != self.name
        elif isinstance(self.reference, Statement):
            return self.reference.name != self.name
        else:
            return False


@dataclass(repr=False)
class InterpSymbol:
    """An interpreted - fully resolved, templated and validated - symbol from Bench source."""

    id: UUID = field(default_factory=uuid.uuid4)
    name: str = field(default="")
    abstract: bool = field(default=False)
    modifier: Optional[StatementModifier] = None
    reference: Optional[InterpSymbol] = None
    definition: Optional[InterpSymbol] = None
    context: OrderedDict[str, "InterpSymbol"] = field(default_factory=OrderedDict)
    source: Optional[Statement] = None

    def to_ref(self) -> InterpSymbol:
        return self.__class__(
            **dict_minus(self.__dict__, ("definition", "source", "id", "reference")),
            reference=self,
            definition=self.definition,
        )

    @property
    def is_definition(self) -> bool:
        return self.definition is not None and self.definition.id == self.id

    @property
    def fqn(self) -> str | None:
        return self.definition.source.fqn if self.definition.source else None

    @property
    def is_root(self):
        return self.source is None or self.source.parent is None

    @property
    def symbol_type(self) -> SymbolType:
        return SYMBOL_TYPE_BY_CLASS[self.__class__]

    @property
    def is_generated(self):
        return self.source is None or self.source.generated

    def __str__(self):
        modifier_str = f"{self.modifier} " if self.modifier else ""
        return f"{modifier_str}{self.symbol_type} {self.name} (source={self.source or '<unknown>'})"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}>"


class SymbolContent:
    def deepcopy(self) -> "SymbolContent":
        # default dataclass copy
        return self.__class__(**self.__dict__)  # type: ignore


# Danger: the order of these types is important because it influences deserialization order.
LiteralValue = Union[
    dict[str, Any], list[Any], bool, int, float, str, PIL.Image.Image, pydub.AudioSegment, None
]
PRIMITIVE_TYPES = [TypeTag.ANY, TypeTag.NULL, TypeTag.BOOLEAN, TypeTag.NUMBER, TypeTag.STRING]


class GeneratedMappingType(enum.StrEnum):
    STATEMENT = "statement"
    RECORD = "record"
    XBLOCK = "xblock"
    TYPE_NODE = "type_node"


@dataclass(repr=False)
class GeneratedMapping:
    """A mapping between a source and a generated target symbol."""

    type: GeneratedMappingType
    source_id: Optional[UUID] = None
    source_revision: Optional[int] = None
    target_id: Optional[UUID] = None
    target_revision: Optional[int] = None

    def __str__(self):
        return f"{self.type} {self.source_id} ({self.source_revision}) -> {self.target_id} ({self.target_revision})"

    def __repr__(self):
        return f"<GeneratedMapping {self}>"


@dataclass(repr=False)
class GeneratorContent:
    generated_mappings: list[GeneratedMapping] = field(default_factory=list)


class TypeNode(abc.ABC):
    name: Optional[str]
    tag: TypeTag
    is_output: bool
    is_nullable: bool
    is_array: bool
    description: Optional[str]
    children: list["TypeNode"]

    @property
    def inputs(self) -> list["TypeNode"]:
        return [child for child in self.children if not child.is_output]

    @property
    def outputs(self) -> list["TypeNode"]:
        return [child for child in self.children if child.is_output]

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

    def deepcopy(self, keep_id: bool = True, keep_reference: bool = True) -> "TypeNode":
        raise NotImplementedError
        # if self.children is not None:
        #     children = [
        #         child.deepcopy(keep_id=keep_id, keep_reference=keep_reference)
        #         for child in self.children
        #     ]
        # else:
        #     children = None
        # return TypeNode(
        #     id=self.id if keep_id else uuid.uuid4(),
        #     name=self.name,
        #     tag=self.tag,
        #     description=self.description,
        #     reference=self.source_reference if not keep_reference else self.reference,
        #     source_reference=self.source_reference,
        #     value=self.value,
        #     children=children,
        # )


@dataclass(repr=False)
class SimpleTypeNode(TypeNode):
    name: Optional[str]
    tag: TypeTag
    order_key: str = INTEGER_ZERO
    id: UUID = field(default_factory=uuid.uuid4)
    description: Optional[str] = None
    is_output: bool = False
    is_array: bool = False
    is_nullable: bool = False
    value: Optional[LiteralValue] = None  # for literal types
    # source reference is separate as the resolved TypeNode may not contain the name
    reference: Union[None, StatementPath, Statement, UUID, "TypeContent", "Type"] = None
    source_reference: Optional[StatementPath] = None

    def __str__(self):
        name_str = f"{self.name} " if self.name else ""
        return f"{name_str}{self.tag}"

    @property
    def children(self) -> list[TypeNode]:
        if isinstance(self.reference, TypeContent):
            return self.reference.type_nodes
        return []


@dataclass(repr=False)
class TypeContent(SymbolContent, TypeNode):
    name: Optional[str] = required_field()
    tag: TypeTag = required_field()
    is_output = False
    is_array = False
    is_nullable = False
    type_nodes: list[SimpleTypeNode] = field(default_factory=list)
    description: Optional[str] = None

    def __str__(self):
        name_str = f"{self.name} " if self.name else ""
        return f"{name_str}{self.tag}"

    @property
    def children(self):
        return self.type_nodes


@dataclass(repr=False)
class Type(InterpSymbol, TypeContent):
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
class TaskContent(SymbolContent, TypeContent, GeneratorContent):
    description: str = ""
    root_type_tag = TypeTag.FUNCTION


@dataclass(repr=False)
class Task(InterpSymbol, TaskContent):
    type: Type = required_field()
    expectations: list[Expectation | Task | Dataset | Code] = field(default_factory=list)
    steps: list[Task | Code] = field(default_factory=list)

    @property
    def generated_expectations(self) -> list[Expectation]:
        return [e for e in self.expectations if e.is_generated]

    @property
    def is_minimally_specified(self) -> bool:
        return bool(self.name and self.type.input.children and self.type.output.children)


@dataclass(repr=False)
class ExpectationContent(SymbolContent):
    description: str

    def __str__(self):
        return f"({self.description})"


@dataclass(repr=False)
class Expectation(InterpSymbol, ExpectationContent):
    expectations: list[Expectation | Task | Dataset | Code] = field(default_factory=list)


@dataclass(repr=False)
class Record:
    order_key: str
    data: typing.Any
    id: UUID = field(default_factory=uuid.uuid4)

    def __str__(self):
        data_type = (
            ",".join(self.data.keys()) if isinstance(self.data, dict) else type(self.data).__name__
        )
        return f"{self.order_key} {data_type}"

    def __repr__(self):
        return f"<Record {self}>"


@dataclass(repr=False)
class DatasetContent(SymbolContent, TypeContent):
    language: Literal["csv"] | Literal["json"] | Literal["jsonl"]
    records: list[Record]
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

    def __len__(self) -> int:
        return len(self.records)


@dataclass(repr=False)
class Dataset(InterpSymbol, DatasetContent):
    type: Type = required_field()


@dataclass(repr=False)
class ModelContent(SymbolContent):
    provider: str
    external_name: str

    def __str__(self):
        return f"provider={self.provider}/{self.external_name}"


@dataclass(repr=False)
class Model(InterpSymbol, ModelContent):
    pass


class XKind(enum.StrEnum):
    Settings = "settings"
    Static = "static"
    Input = "input"
    Output = "output"


class XSource(enum.StrEnum):
    System = "system"
    User = "user"
    Developer = "developer"
    Model = "model"


ValueT = typing.TypeVar("ValueT", bound=typing.Any)


@dataclass(repr=False)
class XBlock(SymbolContent, typing.Generic[ValueT]):
    kind: XKind
    source: XSource
    value: Optional[ValueT]
    path: Optional[str] = None  # jsonpath of value if partial block

    def __len__(self):
        if self.value is None:
            return 0
        elif isinstance(self.value, str):
            return len(self.value)
        else:
            raise TypeError(f"cannot get length of {self}")

    def copy(self):
        return XBlock(
            kind=self.kind,
            source=self.source,
            value=copy.deepcopy(self.value),
            path=self.path,
        )

    def __str__(self):
        return f"{self.value} ({self.kind}/{self.source}, .{self.path or ''})"

    def __repr__(self):
        return f"<XBlock {str(self)}>"


@dataclass(repr=False)
class XBlockContent(XBlock, typing.Generic[ValueT]):
    description: Optional[str] = None
    order_key: str = field(default=INTEGER_ZERO)
    id: UUID = field(default_factory=uuid.uuid4)

    def __str__(self):
        return f"{self.value} ({self.kind}/{self.source}, .{self.path})"

    def __repr__(self):
        return f"<XBlockContent {str(self)}>"


@dataclass(repr=False)
class CodeContent(SymbolContent, GeneratorContent):
    type_node: TypeNode = required_field()
    description: Optional[str] = None
    language: Literal["python"] | Literal["x"] = "python"
    code: Optional[str] = None
    xblocks: Optional[list[XBlockContent]] = field(default_factory=list)

    def __str__(self):
        return f"code={len(self.code)}"


@dataclass(repr=False)
class Code(InterpSymbol, CodeContent):
    type: Type = required_field()


@dataclass(repr=False)
class ProgramContent(SymbolContent):
    pass


@dataclass(repr=False)
class Program(InterpSymbol, ProgramContent):
    pass


@dataclass(repr=False)
class RequirementContent(SymbolContent):
    module_name: Optional[str]
    module_id: Optional[UUID]
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
    builds: list[Build] = field(default_factory=list)


@dataclass(repr=False)
class BuildSettings:
    weights: dict[str, int] = field(default_factory=dict)
    reactive: bool = False


@dataclass(repr=False)
class BuildContent(SymbolContent, GeneratorContent):
    settings: BuildSettings = required_field()
    # TODO @Cleanup: build evaluate settings should be Build.evaluation :BuildEvaluationSettings
    #  But we don't have evaluate as a separate statement in the editor right now.
    evaluate_settings: EvaluateSettings = required_field()
    comment: Optional[str] = None  # like description but non-semantic

    @property
    def targets(self) -> set[UUID]:
        return {mapping.target_id for mapping in self.generated_mappings if mapping.target_id}

    def get_target(self, source_id: UUID) -> Optional[UUID]:
        for mapping in self.generated_mappings:
            if mapping.source_id == source_id:
                return mapping.target_id
        return None

    def __str__(self):
        return ""


@dataclass(repr=False)
class Build(InterpSymbol, BuildContent):
    generated_mappings: list[GeneratedMapping] = field(default_factory=list)
    tasks: list[Task] = field(default_factory=list)
    models: list[Model] = field(default_factory=list)


@dataclass(repr=False)
class EvaluateSettings:
    weights: dict[str, float] = field(default_factory=dict)
    reactive: bool = False


@dataclass(repr=False)
class EvaluateContent(SymbolContent):
    settings: EvaluateSettings
    comment: Optional[str] = None  # like description but non-semantic
    # As noted above, evaluation results are stored elsewhere with other runtime data.


@dataclass(repr=False)
class Evaluate(InterpSymbol, EvaluateContent):
    tasks: list[Task] = field(default_factory=list)
    expectations: list[Expectation | Task | Dataset | Code] = field(default_factory=list)


@dataclass(repr=False)
class BlockContent(SymbolContent):
    pass


@dataclass(repr=False)
class Block(InterpSymbol, BlockContent):
    contents: list[InterpSymbol] = field(default_factory=list)


SYMBOL_CLASS_BY_TYPE: dict[SymbolType, typing.Type[InterpSymbol]] = {
    SymbolType.TYPE: Type,
    SymbolType.CAPABILITY: Capability,
    SymbolType.TASK: Task,
    SymbolType.EXPECTATION: Expectation,
    SymbolType.PROGRAM: Program,
    SymbolType.DATA: Dataset,
    SymbolType.MODEL: Model,
    SymbolType.CODE: Code,
    SymbolType.REQUIREMENT: Requirement,
    SymbolType.RUNCONFIG: Runconfig,
    SymbolType.BUILD: Build,
    SymbolType.EVALUATE: Evaluate,
    SymbolType.BLOCK: Block,
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


def make_func_type(*input_types: TypeNode, output_type: TypeNode, name: str = None) -> TypeNode:
    """Create a function type from input and output types."""
    output_type = output_type.deepcopy()
    output_type.name = "output"
    return TypeNode(
        name=name,
        tag=TypeTag.FUNCTION,
        children=[
            TypeNode(name="input", tag=TypeTag.STRUCT, children=list(input_types)),
            output_type,
        ],
    )


def make_struct_type(*children: TypeNode, name: str = None, description: str = None) -> TypeNode:
    """Create a struct type from children types."""
    return TypeNode(name=name, description=description, tag=TypeTag.STRUCT, children=list(children))


def deepcopy_types(nodes: list[TypeNode] | None, keep_id: bool) -> list[TypeNode]:
    nodes = nodes or []
    return [node.deepcopy(keep_id=keep_id) for node in nodes]


def flatten_func_type(func_type: TypeNode) -> TypeNode:
    """Inline the input and output types into one struct."""
    # check that no input children are called output (hacky deluxe)
    for child in func_type.input.children or []:
        if child.name == "output":
            raise ValueError("input child cannot be named output")
    return SimpleTypeNode(
        name=func_type.name,
        tag=TypeTag.STRUCT,
        type_nodes=[
            *deepcopy_types(func_type.input.children, keep_id=False),
            *deepcopy_types(func_type.output.children, keep_id=False),
        ],
    )
