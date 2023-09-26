import abc
import enum
import typing
import uuid
from dataclasses import dataclass
from typing import Any, Optional, Self, Union
from uuid import UUID

import structlog
from more_itertools import first

from bench.language.const import (
    MNT,
    IssueType,
    StatementReference,
    TypeFlag,
    TypeHint,
    TypeStorageFormat,
    TypeTag,
    new_dynamic_node_key,
)
from bench.language.module import (
    ModuleNode,
    NodeVisitor,
    ScopeNode,
    get_node_id,
    ninternal,
    node,
    node_component,
    nparent,
    nproperty,
)
from bench.language.query import FieldQueryOps
from bench.language.reference import HasReference
from bench.language.text import HasText
from bench.language.value import HasValue
from bench.utils.func import dict_minus
from bench.utils.utils import IdentifierType, to_pyidentifier

if typing.TYPE_CHECKING:
    from bench.language import Statement, Type

logger = structlog.get_logger(__name__)


class TypeError(TypeError):
    def __init__(
        self,
        value: Any,
        expected: "SomeType",
        message: str = None,
        suberrors: list["TypeError"] = None,
    ):
        value_str = repr(value)
        max_value_str_len = 300
        if len(value_str) > max_value_str_len:
            value_str = value_str[: max_value_str_len - 100] + "..." + value_str[-100:]
        super().__init__(
            f"{message or 'type mismatch'}: expected {expected}, got {value_str} ({type(value)})"
        )
        self.value = value
        self.expected = expected
        self.message = message
        self.suberrors = suberrors or []


PRIMITIVE_TYPES = [
    TypeTag.ANY,
    TypeTag.NULL,
    TypeTag.BOOLEAN,
    TypeTag.NUMBER,
    TypeTag.STRING,
    TypeTag.FILE,
    TypeTag.VECTOR,
]
DEFAULT_EMBEDDING_DIMENSION = 1536  # currently only support :FixedEmbeddingDimension
Vector = typing.NewType("Vector", list[float])
Json = typing.NewType("Json", dict)
Key = typing.NewType("Key", str)


@dataclass
class XYPoint:
    """The value of an OpenSearch/GeoJSON-compatible geo_point field."""

    x: float
    y: float


class XYShapeType(enum.StrEnum):
    """The type in an OpenSearch-compatible geo_shape field."""

    POINT = "point"
    LINE_STRING = "line_string"
    POLYGON = "polygon"
    MULTI_POINT = "multi_point"
    MULTI_LINE_STRING = "multi_line_string"
    MULTI_POLYGON = "multi_polygon"
    GEOMETRY_COLLECTION = "geometry_collection"
    ENVELOPE = "envelope"


@dataclass
class XYShape:
    """The value of an OpenSearch-compatible geo_shape field."""

    type: XYShapeType
    coordinates: typing.Union[list[float], list[list[float]]]


TYPE_TAG_BY_TYPE_HINT = {
    # string
    TypeHint.NAME: TypeTag.STRING,
    TypeHint.UUID: TypeTag.STRING,
    TypeHint.DATE: TypeTag.STRING,
    TypeHint.DATETIME: TypeTag.STRING,
    TypeHint.TIME: TypeTag.STRING,
    TypeHint.DURATION: TypeTag.STRING,
    TypeHint.EMAIL: TypeTag.STRING,
    TypeHint.URL: TypeTag.STRING,
    TypeHint.MARKDOWN: TypeTag.STRING,
    TypeHint.RICH_TEXT: TypeTag.STRING,
    TypeHint.HTML: TypeTag.STRING,
    TypeHint.CODE: TypeTag.STRING,
    TypeHint.KEY: TypeTag.STRING,
    TypeHint.PHONE: TypeTag.STRING,
    TypeHint.SECRET: TypeTag.STRING,
    # number
    TypeHint.INTEGER: TypeTag.NUMBER,
    TypeHint.FLOAT: TypeTag.NUMBER,
    TypeHint.SLIDER: TypeTag.NUMBER,
    TypeHint.RATING: TypeTag.NUMBER,
    # boolean
    TypeHint.TOGGLE: TypeTag.BOOLEAN,
    TypeHint.CHECKBOX: TypeTag.BOOLEAN,
    TypeHint.THUMBS: TypeTag.BOOLEAN,
    # file
    TypeHint.IMAGE: TypeTag.FILE,
    TypeHint.VIDEO: TypeTag.FILE,
    TypeHint.AUDIO: TypeTag.FILE,
}

STORAGE_FORMAT_BY_TYPE_TAG = {
    TypeTag.STRING: TypeStorageFormat.STRING,
    TypeTag.JSON: TypeStorageFormat.OBJECT,
    TypeTag.NUMBER: TypeStorageFormat.DOUBLE,
    TypeTag.BOOLEAN: TypeStorageFormat.BOOLEAN,
    TypeTag.VECTOR: TypeStorageFormat.VECTOR,
    TypeTag.FILE: TypeStorageFormat.OBJECT,
    TypeTag.STRUCT: TypeStorageFormat.OBJECT,
    TypeTag.ENUM: TypeStorageFormat.KEYWORD,
    TypeTag.LITERAL: TypeStorageFormat.KEYWORD,
}
STORAGE_FORMAT_BY_TYPE_HINT = {
    # for special types that are not the same as their type tag
    TypeHint.UUID: TypeStorageFormat.KEYWORD,
    TypeHint.DATE: TypeStorageFormat.DATE,
    TypeHint.DATETIME: TypeStorageFormat.DATE,
    TypeHint.TIME: TypeStorageFormat.LONG,
    TypeHint.DURATION: TypeStorageFormat.DOUBLE,
    TypeHint.KEY: TypeStorageFormat.KEYWORD,
    TypeHint.INTEGER: TypeStorageFormat.LONG,
    TypeHint.FLOAT: TypeStorageFormat.DOUBLE,
}


def get_storage_format(tag: TypeTag, hint: TypeHint, flags: TypeFlag) -> TypeStorageFormat:
    # :TypeStorageFormat
    if flags & TypeFlag.IsSecret:
        return TypeStorageFormat.OBJECT  # stored as secret object
    if hint in STORAGE_FORMAT_BY_TYPE_HINT:
        return STORAGE_FORMAT_BY_TYPE_HINT[hint]
    return STORAGE_FORMAT_BY_TYPE_TAG[tag]


class SomeType(abc.ABC):
    """Abstract base for Field nas HasFields/Statement types"""

    id: UUID
    name: Optional[str]
    key: Optional[str]
    tag: TypeTag
    hint: Optional[TypeHint]
    flags: TypeFlag
    text: Optional[str]
    text_plain: Optional[str]
    fields: list["SomeType"]
    resolved_fields: list["SomeType"]  # resolved fields with unions and such
    reference: Union[None, StatementReference, "HasFields"]
    source: Optional["Statement"]

    @property
    def storage_format(self) -> TypeStorageFormat:
        if self.tag == TypeTag.TYPE_REFERENCE and isinstance(self.reference, ModuleNode):
            return self.reference.storage_format
        return get_storage_format(self.tag, self.hint, self.flags)

    @property
    def py_ident(self) -> Optional[str]:
        if self.name is None:
            return None
        elif self.tag == TypeTag.LITERAL:
            return to_pyidentifier(self.name, IdentifierType.CONSTANT)
        else:
            return to_pyidentifier(self.name, IdentifierType.FIELD)

    @property
    def effective_type(self) -> Union["SomeType", "HasFields"]:
        if isinstance(self.reference, HasFields):
            return self.reference
        else:
            return self

    @property
    def effective_tag(self) -> TypeTag:
        return self.effective_type.tag

    @property
    def effective_hint(self) -> Optional[TypeHint]:
        return self.effective_type.hint

    @property
    def inputs(self) -> list["SomeType"]:
        if self.tag != TypeTag.FUNCTION:
            return []
        return [
            child
            for child in (self.resolved_fields or self.fields)
            if not child.flags & TypeFlag.IsOutput and not child.flags & TypeFlag.IsUnionWith
        ]

    @property
    def outputs(self) -> list["SomeType"]:
        if self.tag != TypeTag.FUNCTION:
            return []
        return [
            child
            for child in (self.resolved_fields or self.fields)
            if child.flags & TypeFlag.IsOutput and not child.flags & TypeFlag.IsUnionWith
        ]

    def get_field(self, some_id: str, is_output: bool = None) -> Optional["Field"]:
        for field_ in self.resolved_fields or self.fields:
            if is_output is not None and bool(field_.flags & TypeFlag.IsOutput) != is_output:
                continue
            if field_.py_ident == some_id or field_.name == some_id or field_.key == some_id:
                return field_
        return None

    def has_field(self, some_id: str, is_output: bool = None) -> bool:
        return self.get_field(some_id, is_output=is_output) is not None

    def walk_type(self, path: list["UUID"] | None = None, include_references: bool = False):
        if path is None:
            path = [self.id]
        else:
            path = path + [self.id]
        yield self
        if include_references and self.reference:
            yield from self.reference.walk_type(path, include_references=include_references)
        if self.fields:
            for child in self.fields:
                if child.id in path:
                    continue  # break cycles (allowed, but we don't want to traverse them)
                yield from child.walk_type(path, include_references=include_references)


@node(mnt=MNT.Field)
class Field(HasText, HasValue, HasReference, SomeType, FieldQueryOps):
    parent: Union["Statement", None] = nparent(MNT.Statement)
    name: Optional[str] = nproperty(default=None)
    tag: TypeTag = nproperty()
    hint: Optional[TypeHint] = nproperty(default=None)
    order_key: str | None = ninternal(default=None)
    key: str = nproperty(default=None)
    text: Optional[str] = nproperty(default=None)
    flags: TypeFlag = nproperty(default=TypeFlag.Zero)

    @staticmethod
    def _coerce_from(
        name: str = None, some_type: Union[TypeTag, TypeHint, "Statement"] = None, *args, **kwargs
    ) -> "Field":
        if isinstance(some_type, TypeTag):
            kwargs["tag"] = some_type
        elif isinstance(some_type, TypeHint):
            kwargs["hint"] = some_type
            kwargs["tag"] = TYPE_TAG_BY_TYPE_HINT[some_type]
        elif isinstance(some_type, Statement):
            kwargs["tag"] = TypeTag.TYPE_REFERENCE
            kwargs["reference"] = some_type
        else:
            raise ValueError(f"unexpected type {some_type!r}")
        return Field(name=name, *args, **kwargs)

    def _init_(self):
        self.key = self.key or new_dynamic_node_key(self.ck)

    def __str__(self):
        name_str = f"{self.py_ident} '{self.name}' " if self.name else ""
        return f"{name_str}{self._type_str}"

    def __repr__(self):
        return f"<Field {self}>"

    def __eq__(self, other):
        return FieldQueryOps.__eq__(self, other)  # override to avoid recursion

    @property
    def _type_of_value(self) -> "HasFields":
        from bench.language.libs import symbolx_lib

        return symbolx_lib.lookup_or_error(".reflect.FieldMetadata")

    def _interp_inner(self, scope: ScopeNode) -> None:
        pass  # reference already resolved in HasReference

    def _visit_inner(self, visitor: NodeVisitor) -> None:
        if isinstance(self.reference, ModuleNode):
            visitor.visit_reference(self.reference)

    @property
    def _type_str(self) -> str:
        flag_str = ", ".join(flag.short_name.lower() for flag in TypeFlag if self.flags & flag)
        flags_str = f" ({flag_str})" if flag_str else ""
        if self.hint:
            return f"{self.hint}{flags_str}"
        else:
            return f"{self.tag}{flags_str}"

    @property
    def path(self) -> str:
        if self.parent is None:
            return f"<detached>.{self.py_ident}"
        else:
            return f"{self.parent.path}.{self.py_ident}"

    @property
    def dimensions(self) -> int:
        if self.tag != TypeTag.VECTOR:
            raise ValueError(f"{self} does not have dimensions")
        return (self.value or {}).get("dimensions", DEFAULT_EMBEDDING_DIMENSION)

    @property
    def typed_key(self) -> str:
        if self.storage_format == TypeStorageFormat.VECTOR:
            return f"{self.key}-{self.storage_format.value}{self.dimensions}"
        else:
            return f"{self.key}-{self.storage_format.value}"

    @property
    def source_key(self) -> str:
        return "value." + self.typed_key

    @property
    def resolved_fields(self) -> list["Field"]:
        if isinstance(self.reference, HasFields):
            return self.reference.resolved_fields
        return []

    fields = resolved_fields  # the same by default


@node(mnt=MNT.ResolvedField)
class ResolvedField(Field):
    parent: "Statement" = nparent(MNT.Statement)
    field: Field = ninternal()

    @property
    def field_ck(self) -> UUID:
        return self.field.ck


@node_component(dynamic=True)
class HasFields(SomeType, ModuleNode):
    """A symbol that has fields"""

    def _init(self):
        if self.key is None:
            self.key = new_dynamic_node_key(self.ck)

    def _interp_inner(self, scope: ScopeNode) -> None:
        # expand unions (recursively)
        _resolve_unions(self, [])

    def extend_type(self, *bases: "Type") -> "Self":
        """Adds the fields of another type to this one"""
        for base in bases:
            field_ = Field(
                name=None, tag=TypeTag.TYPE_REFERENCE, reference=base, flags=TypeFlag.IsUnionWith
            )
            self.fields.append(field_)
        return self

    def _inputs_from_args(self, args, kwargs) -> dict:
        inputs = {**kwargs}
        for input_t, input in zip(self.inputs, args):
            inputs[input_t.py_ident] = input
        return inputs


def _resolve_unions(type: "HasFields", path: list[SomeType]) -> None:
    """
    Resolves (and inlines) the union-ed fields of any union types in the type tree.
    """
    if any(n.id == type.id for n in path):
        type._on_issue(
            type=IssueType.CIRCULAR_UNION,
            subject=type,
            path="->".join(n.name for n in path + [type]),
        )
        return  # circular
    if type.resolved_fields is not None:
        return  # already resolved
    if not any(n.flags & TypeFlag.IsUnionWith for n in type.fields):
        type.resolved_fields = type.fields
        return  # skip, not a union
    path = path + [type]

    resolved_fields = []
    for maybe_union in type.fields:
        if not maybe_union.flags & TypeFlag.IsUnionWith:
            resolved_fields.append(maybe_union)
            continue
        if not isinstance(maybe_union.reference, HasFields):
            continue  # ignore unresolved
        _resolve_unions(maybe_union.reference, path)
        if not maybe_union.reference.resolved_fields:
            continue  # couldn't resolve *that* union
        # inline child's type nodes
        for child in maybe_union.reference.resolved_fields:
            existing = first((n for n in resolved_fields if n.name == child.name), None)
            # check if type is compatible if overlapping
            if existing is not None and (
                existing.tag != child.tag
                or existing.flags != child.flags
                or existing.hint != child.hint
            ):
                # TODO @Robustness: check union type compatibility properly/deeply
                type._on_issue(type=IssueType.MISMATCHED_UNION, subject=type, other=existing)
                continue

            # point directly to the field (for transitive unions)
            if isinstance(child, ResolvedField):
                child = child.field

            # derive ck/id
            ck = uuid.uuid5(type.ck, child.ck.hex)
            resolved = ResolvedField(
                ck=ck,
                id=get_node_id(type.module.id, ck),
                parent=type,
                field=child,
                **dict_minus(child.__dict__, ("id", "ck", "field", "parent", "py_type")),
            )
            resolved_fields.append(resolved)
    type.resolved_fields = resolved_fields


class TypedDict(dict):
    """
    A dot dict based on a type.
    Errors on attribute access if the field doesn't exist, otherwise returns the value (or None).
    """

    _PROPS = ("_type", "_is_output")

    def __init__(self, type: "HasFields", d: dict, is_output: bool = None):
        super().__init__(**d)
        self._type = type
        self._is_output = is_output

    def __getattr__(self, item):
        if item in TypedDict._PROPS:
            return super().__getattr__(item)
        try:
            return dict.__getitem__(self, item)
        except KeyError:
            if self._type.has_field(item, is_output=self._is_output):
                return None
            raise AttributeError(item)

    def __setattr__(self, name, value):
        if name in TypedDict._PROPS:
            return super().__setattr__(name, value)
        elif self._type.has_field(name, is_output=self._is_output):
            return dict.__setitem__(self, name, value)
        else:
            raise AttributeError(name)

    def to_dict(self):  # :ToDict
        return self
