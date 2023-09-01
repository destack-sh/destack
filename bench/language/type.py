import abc
import dataclasses
import enum
import inspect
import itertools
import typing
import uuid
from dataclasses import dataclass, field
from datetime import date, datetime, time
from typing import Any, Callable, Collection, Optional, Self, Union
from uuid import UUID

import structlog
from more_itertools import first

from bench.language.const import (
    FieldReferenceMask,
    RemoteObjectStatus,
    StatementType,
    TypeFlag,
    TypeHint,
    TypeStorageFormat,
    TypeTag,
)
from bench.language.core import (
    HasCrud,
    HasSession,
    ModuleNode,
    ModuleVisitor,
    Scope,
    Statement,
    StatementBase,
    StatementPath,
    StatementReference,
    get_node_id,
    node,
)
from bench.language.issue import IssueType
from bench.language.query import FieldQueryOps
from bench.language.remote import RemoteObject, Secret
from bench.utils.fractional import INTEGER_ZERO, generate_n_keys_between
from bench.utils.func import cyrb53a, dict_minus, did_you_mean_str
from bench.utils.utils import DotDict, IdentifierType, required_field, to_pyidentifier

logger = structlog.get_logger(__name__)

PyValueType = Union[int, float, bool, str, dict, list]


class TypeError(TypeError):
    def __init__(
        self,
        value: Any,
        expected: "TypeBase",
        message: str = None,
        suberrors: list["TypeError"] = None,
    ):
        value_str = repr(value)
        max_value_str_len = 400
        if len(value_str) > max_value_str_len:
            value_str = value_str[: max_value_str_len - 100] + "..." + value_str[-100:]
        super().__init__(f"{message or 'type mismatch'}: expected {expected}, got {value_str}")
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
FIELD_KEY_LENGTH = 8

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


class TypeBase(abc.ABC):
    id: UUID
    name: Optional[str]
    key: Optional[str]
    tag: TypeTag
    hint: Optional[TypeHint]
    flags: TypeFlag
    description: Optional[str]
    fields: list["TypeBase"]
    resolved_fields: list["TypeBase"]  # resolved fields with unions and such
    reference: Union[None, StatementReference, "HasType"]
    source: Optional[Statement]

    @property
    def storage_format(self) -> TypeStorageFormat:
        if self.tag == TypeTag.TYPE_REFERENCE and isinstance(self.reference, Type):
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
    def effective_type(self) -> Union["TypeBase", "HasType"]:
        if isinstance(self.reference, HasType):
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
    def bases(self):
        return [field for field in self.fields if field.flags & TypeFlag.IsUnionWith]

    @property
    def inputs(self) -> list["TypeBase"]:
        if self.tag != TypeTag.FUNCTION:
            return []
        return [
            child
            for child in (self.resolved_fields or self.fields)
            if not child.flags & TypeFlag.IsOutput and not child.flags & TypeFlag.IsUnionWith
        ]

    @property
    def outputs(self) -> list["TypeBase"]:
        if self.tag != TypeTag.FUNCTION:
            return []
        return [
            child
            for child in (self.resolved_fields or self.fields)
            if child.flags & TypeFlag.IsOutput and not child.flags & TypeFlag.IsUnionWith
        ]

    def get_field(self, some_id: str) -> Optional["Field"]:
        for field_ in self.resolved_fields or self.fields:
            if field_.py_ident == some_id or field_.name == some_id or field_.key == some_id:
                return field_
        return None

    def has_field(self, some_id: str) -> bool:
        return self.get_field(some_id) is not None

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


def new_field_key(ck: UUID) -> str:
    """
    Gets a 'random' alphabetic key as a persistent key for a field.
    (FIELD_KEY_LENGTH alphabetic characters) :FieldKeys
    """
    hash_value = cyrb53a(str(ck))
    key = ""
    while len(key) < FIELD_KEY_LENGTH:
        hash_value, remainder = divmod(hash_value, 52)
        if remainder < 26:
            key += chr(ord("a") + remainder)
        else:
            key += chr(ord("A") + remainder - 26)
    return key


@node(tracked=["name", "description", "tag", "hint", "flags", "metadata"])
class Field(ModuleNode, HasCrud, HasSession, TypeBase, FieldQueryOps):
    parent: Statement | None = None
    name: Optional[str] = None
    tag: TypeTag = required_field()
    hint: Optional[TypeHint] = None
    order_key: str = INTEGER_ZERO
    key: str = field(default=None)
    description: Optional[str] = None
    flags: TypeFlag = TypeFlag(0)
    metadata: dict[str, Any] = None
    reference: Union[None, StatementPath, Statement, UUID, "Type"] = None
    reference_mask: Union[list[tuple[FieldReferenceMask, str]], None] = None

    def __post_init__(self):
        self.key = self.key or new_field_key(self.ck)

    def __str__(self):
        flag_str = ", ".join(flag.short_name.lower() for flag in TypeFlag if self.flags & flag)
        flags_str = f" ({flag_str})" if flag_str else ""
        name_str = f"{self.py_ident} '{self.name}' " if self.name else ""
        return f"{name_str}{self.tag}{flags_str}"

    def __repr__(self):
        return f"<Field {self}>"

    def __eq__(self, other):
        return FieldQueryOps.__eq__(self, other)  # override to avoid recursion

    def _visit(self, visitor: ModuleVisitor) -> None:
        pass

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
        return (self.metadata or {}).get("dimensions", DEFAULT_EMBEDDING_DIMENSION)

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
        if isinstance(self.reference, Type):
            return self.reference.resolved_fields
        return []

    fields = resolved_fields  # the same by default


@node
class ResolvedField(Field):
    parent: Statement = required_field()
    field: Field = required_field()

    @property
    def field_ck(self) -> UUID:
        return self.field.ck


@node
class Mapping:
    """Mapping fields between statements (or other keyed connections)."""

    connections: Optional[list[tuple[str, str]]] = None


class _FieldAccessor:
    """Access the fields of a type as attributes."""

    def __init__(self, type: "HasType"):
        self.type = type

    def __getattr__(self, item: str):
        return self.type.get_field(item)


@node
class HasType(TypeBase, StatementBase):
    """A symbol that has (but may not be) a type"""

    type: StatementType = StatementType.TYPE
    tag: TypeTag = required_field()
    hint: Optional[TypeHint] = None
    flags: TypeFlag = TypeFlag.Zero
    fields: list[Field] = field(default_factory=list)
    resolved_fields: list[Field | ResolvedField] | None = None
    key: str = None
    reference = None
    _fields_by_ident: dict[str, Field] | None = None

    def _clear(self) -> None:
        self.resolved_fields = None
        self._fields_by_ident = None

    def _interp(self, scope: Scope) -> None:
        # sort fields by order key
        self.fields.sort(key=lambda f: f.order_key)

        # index fields
        self._fields_by_ident = {}
        for field_ in self.fields:
            if field_.py_ident in self._fields_by_ident:
                self._on_issue(
                    type=IssueType.AMBIGUOUS_DEFINITION, subject=self, path=field_.py_ident
                )
                continue
            self._fields_by_ident[field_.py_ident] = field_

        # resolve references
        for n in self.walk_type():
            if n.tag != TypeTag.TYPE_REFERENCE or isinstance(n.reference, Statement):
                continue  # nothing to resolve
            statement = None
            if n.reference is not None:
                statement = scope.lookup(n.reference, statement_t=Type)
            if not isinstance(statement, TypeBase):
                self._on_issue(
                    type=IssueType.MISSING_REFERENCE, subject=self, path=n.name or "<root>"
                )
                continue
            n.reference = statement

        # expand unions (recursively)
        _resolve_unions(self, [])

    def extend_type(self, *bases: "Type") -> "Self":
        """Adds the fields of another type to this one"""
        for base in bases:
            field_ = Field(
                parent=self.parent,
                name=None,
                tag=TypeTag.TYPE_REFERENCE,
                reference=base,
                flags=TypeFlag.IsUnionWith,
            )
            self.session.tracer.field_append(self, field_)
            self.fields.append(field_)
            self._notify_added(field_)
        self._reinterp()
        return self

    def add_field(self, *fields_: Field) -> "Self":
        """Adds a field to this type"""
        last_ok = self.fields[-1].order_key if self.fields else None
        oks = generate_n_keys_between(last_ok, None, len(fields_))
        for ok, field_ in zip(oks, fields_):  # noqa shadows dataclass.field
            self.session.tracer.field_append(self, field_)
            if not field_.detached:
                raise ValueError(f"{field_} is already attached to {field_.parent}")
            field_.parent = self
            field_.order_key = ok
            self.fields.append(field_)
            self._notify_added(field_)
        self._reinterp()
        return self

    def _inputs_from_args(self, args, kwargs) -> dict:
        inputs = {**kwargs}
        for input_t, input in zip(self.inputs, args):
            inputs[input_t.name] = input
        return inputs

    def _take_fields_from(self, other: "HasType", reset_id: bool) -> list[Field]:
        """Copies the fields of this type to another type"""
        new_fields = []
        for field_ in other.fields:
            if not field_.id:
                field_._assign_id(self.module.id)
            field_copy = field_.copy()
            field_copy.parent = self
            field_copy.reference = field_.reference
            if reset_id:
                field_copy.id = None
                field_copy.ck = uuid.uuid4()
            self.fields.append(field_copy)
            self._notify_added(field_copy)
            new_fields.append(field_copy)
        self._assign_oks()
        return new_fields

    def _assign_oks(self):
        oks = generate_n_keys_between(None, None, len(self.fields))
        for ok, field_ in zip(oks, self.fields):
            if field_.parent is None:
                field_.parent = self
            elif field_.parent is not self:
                raise ValueError(f"{field_} is already attached to {field_.parent}")
            field_.order_key = ok

    def __getattr__(self, item):
        if item in self._PROPERTIES:  # defined for all module node classes
            return super().__getattr__(item)
        field_ = self.get_field(item)
        if field_ is not None:
            return field_
        if item in self._names_by_py_ident:
            item = self._names_by_py_ident.get(item)
        statement = self._scopes_by_name.get(item)
        if statement is not None:
            return statement
        candidates = {
            **{s: s for s in self._PROPERTIES},
            **{f.py_ident: f for f in self.fields},
            **{s.py_ident: s for s in self._scopes_by_name.values()},
        }
        did_you_mean = did_you_mean_str(candidates, item)
        raise AttributeError(f"{self} has no attribute {item} ({did_you_mean})")

    @property
    def t(self):
        return _FieldAccessor(self)


def _resolve_unions(type: "HasType", path: list[TypeBase]) -> None:
    """
    Resolves (and inlines) the union-ed fields of any union types in the type tree.
    """
    if any(n.id == type.id for n in path):
        type._on_issue(
            type=IssueType.CIRCULAR_UNION,
            subject=type,
            path="->".join(str(n) for n in path + [type]),
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
        if not isinstance(maybe_union.reference, HasType):
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


# avoid circular import because Tag is HasType but Type is HasTags
from bench.language.tag import HasTags  # noqa


@node
class Type(HasType, HasTags, Statement):
    description: Optional[str] = None
    tag: TypeTag = required_field()
    flags: TypeFlag = TypeFlag(0)
    # not directly configurable for types
    hint = None
    reference = None

    def _clear(self) -> None:
        Statement._clear(self)
        HasType._clear(self)

    def _interp(self, scope: Scope) -> None:
        HasType._interp(self, scope)

    def _visit(self, visitor: ModuleVisitor) -> None:
        for n in itertools.chain(self.fields, self.tags):
            visitor.visit(n)

    def __call__(self, *args, **kwargs):
        combined_kwargs = {**kwargs}
        for i in range(len(args)):
            combined_kwargs[self.fields[i].name] = args[i]
        return DotDict(combined_kwargs)

    def __str__(self):
        path_str = f"{self.path} " if self.name else ""
        return f"{path_str}{self.tag}"

    def __repr__(self):
        return f"<Type {self}>"

    def __getattr__(self, item):
        if self._fields_by_ident is not None and item in self._fields_by_ident:
            return self._fields_by_ident[item]
        else:
            return super().__getattr__(item)

    @property
    def py_ident(self) -> str:
        return to_pyidentifier(self.name, IdentifierType.TYPE)

    @staticmethod
    def from_py_type(py_type: Any):
        return type_from_instance_type(py_type)


def on_invalid_raise(
    value: Any, expected: TypeBase, message: str = None, suberrors: list[TypeError] = None
):
    raise TypeError(value, expected, message, suberrors)


def _map_v_noop(value: Any, *args, **kwargs):
    return value


def _map_k_noop(field: Field):
    return field.name, field.name


def _is_arrayable_not_an_array(type: Field, value: Any) -> bool:
    return (
        not isinstance(value, Collection)
        or isinstance(value, str)
        or (
            type.effective_tag == TypeTag.VECTOR
            and isinstance(value, list)
            and (not value or isinstance(value[0], float))
        )
    )


def map_value(
    value: Any,
    type: TypeBase,
    map_v: Callable[[Any, Field, bool], Any] = None,
    map_k: Callable[[Field], tuple[str, str]] = None,
    is_output: bool = None,
    ignore_array: bool = False,
    ignore_outer_map: bool = False,
):
    """Walks the value and reassembles with new keys and values."""
    map_v = map_v or _map_v_noop
    map_k = map_k or _map_k_noop

    if type.flags & TypeFlag.IsArray and not ignore_array:
        if not isinstance(value, Collection) or isinstance(value, str):
            return value  # type error, ignore here
        return [map_value(item, type, map_v, map_k, ignore_array=True) for item in value]
    elif type.flags & TypeFlag.IsArrayable and not ignore_array:
        if _is_arrayable_not_an_array(type, value):
            return map_value(value, type, map_v, map_k, ignore_array=True)
        return [map_value(item, type, map_v, map_k, ignore_array=True) for item in value]
    elif type.effective_tag in PRIMITIVE_TYPES:
        return map_v(value=value, type=type, ignore_array=ignore_array)
    elif type.effective_tag == TypeTag.ENUM:
        return map_v(value=value, type=type, ignore_array=ignore_array)
    elif type.effective_tag == TypeTag.JSON:
        return value  # nothing to do ?
    elif type.effective_tag not in (TypeTag.STRUCT, TypeTag.FUNCTION):
        raise RuntimeError(f"expected struct-like {type} at {value}")
    if not isinstance(value, typing.Mapping) and not dataclasses.is_dataclass(value):
        return value  # type error, ignore here

    # map into a dict
    mapped = {}
    if type.fields and type.resolved_fields is None:
        raise RuntimeError(f"unexpected unresolved type {type}")
    for subtype in type.resolved_fields:
        if subtype.flags & TypeFlag.IsUnionWith:  # unresolved union
            raise RuntimeError(f"unexpected union with {type}->{subtype}")
        if is_output is not None and bool(subtype.flags & TypeFlag.IsOutput) != is_output:
            continue
        source_k, target_k = map_k(subtype)
        if source_k not in value:
            target_value = None
        else:
            target_value = map_value(value[source_k], subtype, map_v, map_k)
        mapped[target_k] = target_value
    if not ignore_outer_map:
        mapped = map_v(value=mapped, type=type, ignore_array=ignore_array)
    return mapped


TYPENAME_SENTINEL = "__typename"  # :TypeSentinel
OMITTED_SENTINEL = "__omitted"  # :OmittedSentinel
REMOTE_OBJECT_TYPENAME = "RemoteObject"
SECRET_TYPENAME = "Secret"
TypeSignature = typing.NamedTuple(
    "TypeSignature", [("tag", TypeTag), ("hint", Optional[TypeHint]), ("flags", TypeFlag)]
)


def check_type(
    value: Any,
    type: TypeBase,
    get_k: Callable[[Field], str] = None,
    on_invalid=on_invalid_raise,
    is_output: bool = None,
    ignore_array: bool = False,
) -> None:
    """
    Checks whether the given value has the expected type (recursively).
    Raises TypeError if not.
    """

    get_k = get_k or (lambda f: f.py_ident)

    def _check(valid: bool, message: str = None):
        if not valid:
            on_invalid(value, type, message)
        return valid

    # optional / list types
    if type.flags & TypeFlag.IsOptional and value is None:
        return
    elif type.flags & TypeFlag.IsArray and not ignore_array:
        if _check(isinstance(value, Collection)):
            for item in value:
                check_type(item, type, get_k=get_k, on_invalid=on_invalid, ignore_array=True)
        return
    elif (
        type.flags & TypeFlag.IsArrayable
        and not ignore_array
        and not _is_arrayable_not_an_array(type, value)
    ):
        for item in value:
            check_type(item, type, get_k=get_k, on_invalid=on_invalid, ignore_array=True)
        return

    # basic instance value check
    mapper = get_type_mapper_by_type(type)
    if not _check(mapper.is_instance_value(type, value)):
        return

    # walk struct-like types
    if type.effective_tag == TypeTag.STRUCT or type.effective_tag == TypeTag.FUNCTION:
        if type.tag == TypeTag.FUNCTION and is_output and not type.outputs:
            value = value or {}  # None is allowed for empty outputs
        is_dataclass = dataclasses.is_dataclass(value)
        for f in type.resolved_fields or type.fields:
            if is_output is not None and bool(f.flags & TypeFlag.IsOutput) != is_output:
                continue
            k = get_k(f)
            if is_dataclass:
                subvalue = getattr(value, k)
            else:
                subvalue = value.get(k)
            check_type(subvalue, f, get_k=get_k, on_invalid=on_invalid)
        if hasattr(value, "keys"):
            for key in value.keys():
                _check(type.has_field(key), f"extraneous field {key}")


class TypeMapper:
    """
    Maps specific types (and values) into and from Python.
    Don't bother with lists and optional types here.
    """

    def is_instance_type(self, py_type: type) -> bool:
        """Whether this mapper can represent the given Python instance type."""
        raise NotImplementedError

    def from_instance_type(self, py_type: type, type_map: dict[type, Any]) -> Type:
        """Converts a Python instance type into a Bench Type."""
        raise NotImplementedError

    def is_instance_value(self, type: TypeBase, value: Any) -> bool:
        """
        Whether this mapper can represent the given Python instance value.
        For nested types (like structs) this only checks the top-level value (no walking).
        """
        raise NotImplementedError

    def to_instance_value(self, type: TypeBase, value: Any) -> Any:
        """Converts a value of the given flat type into an instance value."""
        return value

    def to_flat_value(self, type: TypeBase, value: Any) -> Any:
        """Converts a value of the given flat type into a flat value."""
        return value


type_mappers: dict[TypeSignature, TypeMapper] = {}


def register_mapper(
    mapping: TypeMapper,
    *,
    tags: list[TypeTag] = None,
    hints: list[TypeHint] = None,
    flags: TypeFlag = None,
):
    if not tags and not hints:
        raise ValueError("at least one tag or hint must be specified")

    def _register(signature: TypeSignature):
        if signature in type_mappers:
            raise ValueError(
                f"mapper for {signature} already registered: {type_mappers[signature]}"
            )
        type_mappers[signature] = mapping

    tags = tags or []
    hints = hints or []
    flags = flags or TypeFlag.Zero
    for tag in tags:
        _register(TypeSignature(tag, None, flags))
    for hint in hints:
        tag = TYPE_TAG_BY_TYPE_HINT[hint]
        _register(TypeSignature(tag, hint, flags))


def get_type_mapper_by_type(type: TypeBase) -> TypeMapper:
    """
    Gets the most appropriate mapping for the given type.
    (flat because we ignore list and optional types).
    """
    # strip to only relevant flags for mapping
    stripped_flags = type.flags & TypeFlag.IsSecret
    exact_signature = TypeSignature(type.effective_tag, type.effective_hint, stripped_flags)
    mapping = type_mappers.get(exact_signature)
    if mapping is not None:
        return mapping
    # no exact match, try generic without hint
    stripped_signature = TypeSignature(type.effective_tag, None, stripped_flags)
    mapping = type_mappers.get(stripped_signature)
    if mapping is not None:
        return mapping
    raise LookupError(f"no mapping found for {type}")


def get_type_mapper_by_instance_type(py_type: type) -> tuple[TypeMapper, type, TypeFlag]:
    """
    Gets the most appropriate mapping for the given Python type.
    (flat because we "ignore" list and optional types (inside the mapper)).
    """
    py_type, flags = _strip_py_type(py_type)
    # get mapping
    for mapping in type_mappers.values():
        if mapping.is_instance_type(py_type):
            return mapping, py_type, flags
    raise LookupError(f"no mapping found for {py_type} ({flags}, type={type(py_type)})")


def _strip_py_type(py_type: type) -> tuple[type, TypeFlag]:
    flags = TypeFlag.Zero
    # strip optional
    if typing.get_origin(py_type) is typing.Union:
        args = typing.get_args(py_type)
        if len(args) == 2 and args[1] == type(None):  # noqa: E721
            py_type = args[0]
            flags |= TypeFlag.IsOptional
        # convert x | list[x] as isarrayable
        elif len(args) == 2 and typing.get_origin(args[1]) is list:
            if args[0] != typing.get_args(args[1])[0]:
                raise ValueError(f"cannot map generic union types: {py_type}")
            py_type = args[0]
            flags |= TypeFlag.IsArrayable
        else:
            raise ValueError(f"cannot map generic union types: {py_type}")
    # strip list
    if typing.get_origin(py_type) is list:
        py_type = typing.get_args(py_type)[0]
        flags |= TypeFlag.IsArray
    return py_type, flags


@dataclass
class StaticPyTypeMapper(TypeMapper):
    py_type: type
    py_type_raw: type = field(init=False)
    tag: TypeTag
    hint: Optional[TypeHint] = None

    def __post_init__(self):
        self.py_type_raw = self.py_type
        # strip newtype
        if hasattr(self.py_type, "__supertype__"):
            self.py_type_raw = self.py_type.__supertype__

    def is_instance_type(self, py_type: type) -> bool:
        return py_type == self.py_type

    def from_instance_type(self, py_type: type, type_map: dict[type, Any]) -> Type:
        return Type(name=None, tag=self.tag, hint=self.hint)

    def is_instance_value(self, type: TypeBase, value: Any) -> bool:
        return isinstance(value, self.py_type_raw)

    def to_instance_value(self, type: TypeBase, value: Any) -> Any:
        return self.py_type(value)


@dataclass
class VectorTypeMapper(StaticPyTypeMapper):
    py_type: type = Vector
    tag: TypeTag = TypeTag.VECTOR

    def is_instance_value(self, type: TypeBase, value: Any) -> bool:
        # not quite right but good enough for now
        return isinstance(value, list) and len(value) > 0 and isinstance(value[0], float)


@dataclass
class StringifyTypeMapping(StaticPyTypeMapper):
    def is_instance_type(self, py_type: type) -> bool:
        return self.py_type == py_type

    def to_instance_value(self, type: TypeBase, value: Any) -> Any:
        return self.py_type(value)

    def to_flat_value(self, type: TypeBase, value: Any) -> str:
        return str(value)


class IsoDtTypeMapping(StaticPyTypeMapper):
    HINT_BY_PY_TYPE = {
        date: TypeHint.DATE,
        datetime: TypeHint.DATETIME,
        time: TypeHint.TIME,
    }

    def is_instance_type(self, py_type: type) -> bool:
        return inspect.isclass(py_type) and any(
            issubclass(py_type, t) for t in self.HINT_BY_PY_TYPE
        )

    def to_instance_value(self, type: TypeBase, value: Any) -> Any:
        return self.py_type.fromisoformat(value)

    def from_instance_type(self, py_type: type, type_map: dict[type, Any]) -> Type:
        return Type(tag=TypeTag.STRING, hint=self.HINT_BY_PY_TYPE[py_type])

    def to_flat_value(self, type: TypeBase, value: Any) -> str:
        return value.isoformat()


class EnumMapper(TypeMapper):
    def is_instance_type(self, py_type: type) -> bool:
        return inspect.isclass(py_type) and issubclass(py_type, enum.StrEnum)

    def from_instance_type(self, py_type: type, type_map: dict[type, Any]) -> TypeBase:
        assert issubclass(py_type, enum.StrEnum)
        if py_type in type_map:
            return type_map[py_type]
        type = Type(name=py_type.__name__, tag=TypeTag.ENUM)
        type_map[py_type] = type
        for py_member in py_type.__members__.values():
            member = Field(name=py_member.name, key=py_member.name, tag=TypeTag.LITERAL)
            type.fields.append(member)
        type._assign_oks()
        return type

    def is_instance_value(self, type: TypeBase, value: Any) -> bool:
        if isinstance(value, str):
            # allow string values for built-in enums
            # (that also function as regular enums in code)
            return type.has_field(value)
        return isinstance(value, Field) and type.has_field(value.key)

    def to_instance_value(self, type: Type, value: Any) -> Any:
        field_ = type.get_field(value)
        return field_.name if field_ else value

    def to_flat_value(self, type: Type, value: Any) -> Any:
        field_ = type.get_field(value) if not isinstance(value, Field) else value
        return field_.key if field_ else value


class FileMapper(TypeMapper):
    def is_instance_type(self, py_type: type) -> bool:
        return py_type is RemoteObject

    def from_instance_type(self, py_type: type, type_map: dict[type, Any]) -> TypeBase:
        return Type(name=None, tag=TypeTag.FILE)

    def is_instance_value(self, type: TypeBase, value: Any) -> bool:
        return isinstance(value, RemoteObject)

    def to_instance_value(self, type: TypeBase, value: Any) -> Any:
        return RemoteObject(
            id=UUID(value["id"]),
            name=value["name"],
            content_type=value["content_type"],
            content_length=value["content_length"],
            sha512=value["sha512"],
            status=RemoteObjectStatus[value["status"]],
        )

    def to_flat_value(self, type: TypeBase, value: Any) -> Any:
        return {
            TYPENAME_SENTINEL: REMOTE_OBJECT_TYPENAME,
            "id": str(value.id),
            "name": value.name,
            "content_type": value.content_type,
            "content_length": value.content_length,
            "sha512": value.sha512,
            "status": value.status.name,
        }


class SecretTypeMapper(TypeMapper):
    def is_instance_type(self, py_type: type) -> bool:
        return py_type is Secret

    def from_instance_type(self, py_type: type, type_map: dict[type, Any]) -> TypeBase:
        return Type(name=None, tag=TypeTag.STRING, hint=TypeHint.SECRET, flags=TypeFlag.IsSecret)

    def is_instance_value(self, type: TypeBase, value: Any) -> bool:
        return isinstance(value, Secret)

    def to_instance_value(self, type: TypeBase, value: Any) -> Any:
        return Secret(
            id=UUID(value["id"]),
            sha512=value["sha512"],
        )

    def to_flat_value(self, type: TypeBase, value: Any) -> Any:
        return {
            TYPENAME_SENTINEL: SECRET_TYPENAME,
            "id": str(value.id),
            "sha512": value.sha512,
        }


class StructTypeMapper(TypeMapper):
    def is_instance_type(self, py_type: type) -> bool:
        return dataclasses.is_dataclass(py_type) or typing.is_typeddict(py_type)

    def from_instance_type(self, py_type: type, type_map: dict[str, Any]) -> Type:
        if py_type in type_map:
            return type_map[py_type]
        type = Type(name=py_type.__name__, tag=TypeTag.STRUCT)
        type_map[py_type] = type
        if dataclasses.is_dataclass(py_type):
            for py_field in dataclasses.fields(py_type):
                field_ = field_from_instance_type(py_field.type, py_field.name, type_map)
                type.fields.append(field_)
        elif typing.is_typeddict(py_type):
            for py_field_name, py_field in typing.get_type_hints(py_type).items():
                field_ = field_from_instance_type(py_field, py_field_name, type_map)
                type.fields.append(field_)
        else:
            raise ValueError(f"unsupported struct type: {py_type}")
        type._assign_oks()
        return type

    def is_instance_value(self, type: TypeBase, value: Any) -> bool:
        return isinstance(value, dict) or dataclasses.is_dataclass(value)

    def to_instance_value(self, type: TypeBase, value: Any) -> Any:
        return DotDict(value)

    def to_flat_value(self, type: TypeBase, value: Any) -> Any:
        return {TYPENAME_SENTINEL: type.key, **value}


class JsonTypeMapper(TypeMapper):
    def is_instance_type(self, py_type: type) -> bool:
        return py_type is Json or py_type is dict or typing.get_origin(py_type) is dict

    def is_instance_value(self, type: TypeBase, value: Any) -> bool:
        return True  # not sure how to check this

    def from_instance_type(self, py_type: type, type_map: dict[type, Any]) -> Type:
        return Type(name=None, tag=TypeTag.JSON)


class FunctionTypeMapper(TypeMapper):
    def is_instance_type(self, py_type: type) -> bool:
        return inspect.isfunction(py_type)

    def is_instance_value(self, type: TypeBase, value: Any) -> bool:
        return value is None or isinstance(value, dict)

    def from_instance_type(self, py_type: type, type_map: dict[type, Any]) -> Type:
        if py_type in type_map:
            return type_map[py_type]
        type = Type(name=py_type.__name__, tag=TypeTag.FUNCTION)
        type_map[py_type] = type
        signature = inspect.signature(py_type)
        for py_param in signature.parameters.values():
            if py_param.name == "self" and (
                py_param.annotation is py_param.empty
                or py_param.annotation.__name__ is py_type.__name__
            ):
                continue
            param = field_from_instance_type(py_param.annotation, py_param.name, type_map)
            type.fields.append(param)

        # output must be a struct, inline it with output flag
        if signature.return_annotation is inspect.Signature.empty:
            raise ValueError(f"missing return annotation for {py_type}")
        output = type_from_instance_type(signature.return_annotation, None, type_map)
        if output.tag != TypeTag.STRUCT:
            raise ValueError(f"function output must be a struct: {py_type}")
        for output_field in type._take_fields_from(output, reset_id=True):
            output_field.flags |= TypeFlag.IsOutput

        type._assign_oks()
        return type


_TYPE_MAP: dict[Any, Type] = {}


def type_from_instance_type(
    py_type: type, name: Optional[str], type_map: dict[Any, Type] = None
) -> "Type":
    """
    Maps a python type to a Type (recursively).
    Nested types are read/written in the given type_map.
    Types are keyed by name since we have no way to associate keys over time.
    """
    type_map = type_map or _TYPE_MAP
    map, stripped, flags = get_type_mapper_by_instance_type(py_type)
    type = map.from_instance_type(stripped, type_map)
    if name:
        type.name = name
    type.flags |= flags
    return type


def field_from_instance_type(py_type: type | str, name: str, type_map: dict[Any, Type]) -> Field:
    name_nice = name.replace("_", " ")
    if to_pyidentifier(name_nice, IdentifierType.FIELD) != name:
        raise ValueError(f"inconsistent field name: {name} != {name_nice}")

    stripped, flags = _strip_py_type(py_type)
    if isinstance(stripped, str):
        # lookup by name in type_map
        type = first((t for k, t in type_map.items() if k.__name__ == stripped), None)
        if type is None:
            raise ValueError(f"unknown type name: {stripped}")
    elif isinstance(stripped, typing.ForwardRef):
        # lookup by name in type_map
        type = first(
            (t for k, t in type_map.items() if k.__name__ == stripped.__forward_arg__), None
        )
        if type is None:
            raise ValueError(f"unknown type name: {stripped}")
    elif stripped in type_map:
        type = type_map[stripped]
    else:
        type = type_from_instance_type(py_type, name, type_map)
    # key is set to None so we error if they're not set later
    if type.tag in (TypeTag.STRUCT, TypeTag.ENUM, TypeTag.TYPE_REFERENCE):
        return Field(
            name=name_nice, key=None, tag=TypeTag.TYPE_REFERENCE, reference=type, flags=flags
        )
    else:
        return Field(name=name_nice, key=None, tag=type.tag, hint=type.hint, flags=flags)


def instantiate_value_flat(value: Any, type: TypeBase, ignore_array: bool = False) -> Any:
    """Maps to the proper Python representation of the given value."""
    if value is None:  # skip null values
        return None  # type checking is done elsewhere
    # auto coerce lists to element and vice versa (like in frontend) :ArrayCoercion
    mapping = get_type_mapper_by_type(type)
    try:
        if type.flags & TypeFlag.IsArrayable:  # keep as is
            if not isinstance(value, list):
                return mapping.to_instance_value(type, value)
            else:
                return [mapping.to_instance_value(type, v) for v in value]
        elif type.flags & TypeFlag.IsArray and not ignore_array:  # promote to array
            if not isinstance(value, list):
                value = [value]
            else:
                return [mapping.to_instance_value(type, v) for v in value]
        else:  # trim to element
            if isinstance(value, list):
                value = value[0]
            else:
                return mapping.to_instance_value(type, value)
    except (KeyError, ValueError, TypeError):
        logger.warning("instantiate_failed", exc_info=True, value=value, type=type)
        return value  # type checking is done elsewhere


def strip_value_flat(value: Any, type: TypeBase, *args, **kwargs) -> Any:
    """Maps back to the raw value from the Python representation."""
    # we don't auto-coerce here since that's only needed for external data
    if value is None:
        return None
    mapping = get_type_mapper_by_type(type)
    if not mapping.is_instance_value(type, value):
        return None  # type-checking is done elsewhere
    return mapping.to_flat_value(type, value)


def instantiate_value(
    value: Any,
    type: HasType,
    ignore_array: bool = False,
    ignore_outer_map: bool = False,
    is_output: bool = None,
):
    return map_value(
        value=value,
        type=type,
        map_k=lambda f: (f.typed_key, f.py_ident),
        map_v=instantiate_value_flat,
        ignore_array=ignore_array,
        ignore_outer_map=ignore_outer_map,
        is_output=is_output,
    )


def strip_value(
    value: Any,
    type: HasType,
    ignore_array: bool = False,
    ignore_outer_map: bool = False,
    is_output: bool = None,
):
    return map_value(
        value=value,
        type=type,
        map_k=lambda f: (f.py_ident, f.typed_key),
        map_v=strip_value_flat,
        ignore_array=ignore_array,
        ignore_outer_map=ignore_outer_map,
        is_output=is_output,
    )


# type tags
register_mapper(StaticPyTypeMapper(str, TypeTag.STRING), tags=[TypeTag.STRING])
register_mapper(StaticPyTypeMapper(Key, TypeTag.STRING, TypeHint.KEY), hints=[TypeHint.KEY])
register_mapper(StaticPyTypeMapper(float, TypeTag.NUMBER), tags=[TypeTag.NUMBER])
register_mapper(StaticPyTypeMapper(type(None), TypeTag.NULL), tags=[TypeTag.NULL])
register_mapper(StaticPyTypeMapper(bool, TypeTag.BOOLEAN), tags=[TypeTag.BOOLEAN])
register_mapper(VectorTypeMapper(), tags=[TypeTag.VECTOR])
register_mapper(FileMapper(), tags=[TypeTag.FILE])
register_mapper(EnumMapper(), tags=[TypeTag.ENUM])
register_mapper(StructTypeMapper(), tags=[TypeTag.STRUCT])
register_mapper(FunctionTypeMapper(), tags=[TypeTag.FUNCTION])
register_mapper(JsonTypeMapper(), tags=[TypeTag.JSON])
# type hints
register_mapper(StringifyTypeMapping(UUID, TypeTag.STRING, TypeHint.UUID), hints=[TypeHint.UUID])
register_mapper(IsoDtTypeMapping(date, TypeTag.STRING), hints=[TypeHint.DATE])
register_mapper(IsoDtTypeMapping(datetime, TypeTag.STRING), hints=[TypeHint.DATETIME])
register_mapper(IsoDtTypeMapping(time, TypeTag.STRING), hints=[TypeHint.TIME])
register_mapper(StaticPyTypeMapper(int, TypeTag.NUMBER, TypeHint.INTEGER), hints=[TypeHint.INTEGER])
# other
register_mapper(SecretTypeMapper(), tags=[TypeTag.STRING, TypeTag.NUMBER], flags=TypeFlag.IsSecret)
