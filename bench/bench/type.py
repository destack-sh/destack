import abc
import dataclasses
import enum
import inspect
import random
import string
import typing
import uuid
from dataclasses import dataclass, field
from datetime import date, datetime, time
from functools import cached_property
from typing import Any, Callable, Collection, Mapping, Optional, Self, Union
from uuid import UUID, uuid4

import structlog
from more_itertools import first

from bench.bench.const import RemoteObjectStatus, TypeFlag, TypeHint, TypeTag
from bench.bench.core import (
    HasCrud,
    HasSession,
    ModuleNode,
    Scope,
    Statement,
    StatementPath,
    StatementReference,
    StatementType,
    Symbol,
    SymbolBase,
    node,
)
from bench.bench.expect import HasExpectations
from bench.bench.issue import IssueType
from bench.bench.remote import RemoteObject, Secret
from bench.utils.fractional import INTEGER_ZERO, generate_n_keys_between
from bench.utils.func import dict_minus
from bench.utils.utils import required_field, to_pyidentifier

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
        super().__init__(f"{message or 'type mismatch'}: expected {expected}, got {value}")
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


class TypeStorageFormat(enum.StrEnum):
    """
    The fundamental form of a field/type.
    Since we're using OpenSearch for our user data backend, this
    needs to be compatible with OpenSearch's field types.
    However, be mindful of other future storage backends.
    :TypeStorageFormat
    TODO @Architecture: consider consolidating type storage/tage/hint somehow
    """

    STRING = "str"
    DOUBLE = "f64"
    LONG = "s64"
    VECTOR = "vec"
    BINARY = "bin"
    BOOLEAN = "bool"
    DATE = "date"
    KEYWORD = "key"
    OBJECT = "obj"
    RELATION = "rel"


STORAGE_FORMAT_BY_TYPE_TAG = {
    TypeTag.STRING: TypeStorageFormat.STRING,
    TypeTag.JSON: TypeStorageFormat.OBJECT,
    TypeTag.NUMBER: TypeStorageFormat.DOUBLE,
    TypeTag.BOOLEAN: TypeStorageFormat.BOOLEAN,
    TypeTag.VECTOR: TypeStorageFormat.VECTOR,
    TypeTag.FILE: TypeStorageFormat.OBJECT,
    TypeTag.STRUCT: TypeStorageFormat.OBJECT,
    TypeTag.ENUM: TypeStorageFormat.KEYWORD,
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

    @cached_property
    def storage_format(self) -> TypeStorageFormat:
        if self.tag == TypeTag.TYPE_REFERENCE and isinstance(self.reference, Type):
            return self.reference.storage_format
        return get_storage_format(self.tag, self.hint, self.flags)

    @property
    def ident(self):
        return to_pyidentifier(self.name)

    @property
    def effective_type(self) -> Union["TypeBase", "HasType"]:
        if isinstance(self.reference, HasType):
            return self.reference
        else:
            return self

    @property
    def bases(self):
        return [field for field in self.fields if field.flags & TypeFlag.IsUnionWith]

    @property
    def inputs(self) -> list["TypeBase"]:
        return [child for child in self.fields if not child.flags & TypeFlag.IsOutput]

    @property
    def outputs(self) -> list["TypeBase"]:
        return [child for child in self.fields if child.flags & TypeFlag.IsOutput]

    def __getitem__(self, item: str) -> "TypeBase":
        node = first(
            (
                child
                for child in self.fields
                if child.name == item or child.ident == item or child.key == item
            ),
            None,
        )
        if node is None:
            raise KeyError(item)
        return node

    def __contains__(self, item):
        return any(
            child
            for child in self.fields
            if child.name == item or child.ident == item or child.key == item
        )

    def walk_type(self, path: list["TypeBase"] | None = None, include_references: bool = False):
        if path is None:
            path = [self]
        else:
            path = path + [self]
        yield self
        if include_references and self.reference:
            yield from self.reference.walk_type(path, include_references=include_references)
        if self.fields:
            for child in self.fields:
                if child in path:
                    continue  # break cycles (allowed, but we don't want to traverse them)
                yield from child.walk_type(path, include_references=include_references)

    def unkey(self, data: Any, is_output: bool = None, to_ident: bool = False) -> Any:
        """'Unkeys' data by replacing keys with the names of the type nodes."""
        return unkey_value(data, self, is_output=is_output, to_ident=to_ident)

    def rekey(self, data: Any, is_output: bool = None, via_ident: bool = False) -> Any:
        """'Keys' data by replacing names with the keys of the type nodes."""
        return rekey_value(data, self, is_output=is_output, from_ident=via_ident)


def new_field_key() -> str:
    """Gets a random alphabetic key as a persistent key for a type node."""
    # (upper and lower case letters only)
    # :TypeNodeKeys
    return "".join(random.choices(string.ascii_letters, k=FIELD_KEY_LENGTH))


@node(tracked=["name", "description", "tag", "hint", "flags", "metadata"])
class Field(ModuleNode, HasCrud, HasSession, TypeBase):
    parent: Statement | None = None
    name: Optional[str] = None
    tag: TypeTag = required_field()
    hint: Optional[TypeHint] = None
    order_key: str = INTEGER_ZERO
    key: str = field(default_factory=new_field_key)
    description: Optional[str] = None
    flags: TypeFlag = TypeFlag(0)
    metadata: dict[str, Any] = None
    reference: Union[None, StatementPath, Statement, UUID, "Type"] = None

    def __str__(self):
        name_str = f"{self.name} " if self.name else ""
        return f"{name_str}{self.tag}"

    def __repr__(self):
        return f"<Field {self}>"

    @property
    def dimensions(self) -> int:
        if self.tag != TypeTag.VECTOR:
            raise ValueError(f"{self} does not have dimensions")
        return (self.metadata or {}).get("dimensions", DEFAULT_EMBEDDING_DIMENSION)

    @cached_property
    def typed_key(self) -> str:
        if self.storage_format == TypeStorageFormat.VECTOR:
            return f"{self.key}-{self.storage_format.value}{self.dimensions}"
        else:
            return f"{self.key}-{self.storage_format.value}"

    @property
    def resolved_fields(self) -> list["Field"]:
        if isinstance(self.reference, Type):
            return self.reference.fields
        return []

    fields = resolved_fields  # the same by default


@node
class ResolvedField(Field):
    parent: Statement = required_field()
    field: Field = required_field()


@node
class HasType(TypeBase, SymbolBase):
    """A symbol that has (but is not) a type"""

    tag: TypeTag = required_field()
    hint: Optional[TypeHint] = None
    flags: TypeFlag = TypeFlag.Zero
    fields: list[Field] = field(default_factory=list)
    resolved_fields: list[Field | ResolvedField] | None = None
    key: str = None
    reference = None

    def _clear(self) -> None:
        self.resolved_fields = None

    def _interp(self, scope: Scope) -> None:
        # resolve references
        for n in self.walk_type():
            if n.tag != TypeTag.TYPE_REFERENCE or isinstance(n.reference, Symbol):
                continue  # nothing to resolve
            if n.reference is None:
                symbol = None
            else:
                symbol = scope.lookup_symbol(n.reference, StatementType.TYPE)
            if not isinstance(symbol, TypeBase):
                self._on_issue(
                    type=IssueType.MISSING_REFERENCE, subject=self, path=n.name or "<root>"
                )
                continue
            n.reference = symbol

        # expand unions (recursively)
        Type._resolve_unions(self, [])

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
        self._reinterp()
        return self

    def add_field(self, *fields_: Field) -> "Self":
        """Adds a field to this type"""
        last_ok = self.fields[-1].order_key if self.fields else None
        oks = generate_n_keys_between(last_ok, None, len(fields_))
        for ok, field_ in zip(oks, fields_):  # noqa shadows dataclass.field
            self.session.tracer.field_append(self, field_)
            field_.parent = self
            field_.order_key = ok
            self.fields.append(field_)
        self._reinterp()
        return self

    @property
    def type(self) -> TypeBase:
        """For clarity when explicitly referring to the type of a symbol"""
        return self

    @staticmethod
    def _resolve_unions(type: "Type", path: list[TypeBase]) -> None:
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
            if not isinstance(maybe_union.reference, Type):
                continue  # ignore unresolved
            # inline child's type nodes
            Type._resolve_unions(maybe_union.reference, path)
            if not maybe_union.reference.resolved_fields:
                continue  # couldn't resolve
            for child in maybe_union.reference.resolved_fields:
                existing = first((n for n in resolved_fields if n.name == child.name), None)
                # check if type is compatible if overlapping
                if existing is not None and (
                    existing.tag != child.tag
                    or existing.flags != child.flags
                    or existing.hint != child.hint
                ):
                    # TODO @Robustness: check union type compatibility properly/deeply
                    type._on_issue(
                        type=IssueType.MISMATCHED_UNION,
                        subject=type,
                        path="->".join(str(n) for n in path),
                    )
                    continue
                resolved = ResolvedField(
                    id=uuid.uuid5(child.id, type.id.hex),
                    parent=type,
                    field=child,
                    **dict_minus(child.__dict__, ("id", "field", "parent")),
                )
                resolved_fields.append(resolved)
        type.resolved_fields = resolved_fields


@node
class Type(Symbol, HasType, HasExpectations):
    description: Optional[str] = None
    tag: TypeTag = required_field()
    flags: TypeFlag = TypeFlag(0)
    # not directly configurable for types
    hint = None
    key = None
    reference = None
    _fields_by_ident: dict[str, Field] | None = None

    @cached_property
    def py_type(self) -> type | enum.Enum:
        return instantiate_py_type(self, self.session)

    def _clear(self) -> None:
        Symbol._clear(self)
        HasType._clear(self)
        HasExpectations._clear(self)
        self._fields_by_ident = None

    def _interp(self, scope: Scope) -> None:
        HasType._interp(self, scope)
        HasExpectations._interp(self, scope)
        self._fields_by_ident = {}
        for field_ in self.fields:
            self._fields_by_ident[field_.ident] = field_

    def __call__(self, *args, **kwargs):
        return self.py_type(*args, **kwargs)

    def __str__(self):
        name_str = f"{self.name} " if self.name else ""
        return f"{name_str}{self.tag}"

    def __repr__(self):
        return f"<Type {self}>"

    def __getattr__(self, item):
        if self._fields_by_ident is not None and item in self._fields_by_ident:
            return self._fields_by_ident[item]
        else:
            return super().__getattr__(item)

    def _assign_oks(self):
        oks = generate_n_keys_between(None, None, len(self.fields))
        for ok, field_ in zip(oks, self.fields):
            field_.order_key = ok

    @staticmethod
    def from_py_type(py_type: Any):
        return type_from_py_type(py_type)


def on_invalid_raise(
    value: Any, expected: TypeBase, message: str = None, suberrors: list[TypeError] = None
):
    raise TypeError(value, expected, message, suberrors)


def check_type(
    value: Any,
    expected: TypeBase,
    eager_error: bool = True,
    on_invalid=on_invalid_raise,
    ignore_array: bool = False,
    is_output: bool = None,
):
    """
    Checks whether the given value has the expected type (recursively).
    Raises TypeError if not at the first issue.
    """

    _suberrors = []

    def _on_invalid_collect(
        value: Any,
        expected: TypeBase,
        message: str = None,
        suberrors: list[TypeError] = None,
    ):
        _suberrors.append(TypeError(value, expected, message, suberrors))

    def _check(valid: bool, message: str):
        if not valid:
            if eager_error:
                on_invalid(value, expected, message)
            else:
                _on_invalid_collect(value, expected, message)
        return valid

    if expected.flags & TypeFlag.IsArray and not ignore_array:
        if _check(isinstance(value, Collection), "expected array"):
            for item in value:
                check_type(
                    item,
                    expected,
                    eager_error=eager_error,
                    on_invalid=on_invalid,
                    ignore_array=True,
                )
    elif expected.flags & TypeFlag.IsSecret:
        _check(isinstance(value, Secret), "expected secret")
    elif expected.tag == TypeTag.STRING:
        _check(isinstance(value, str), "expected string")
    elif expected.tag == TypeTag.NUMBER:
        _check(isinstance(value, (int, float)), "expected number")
    elif expected.tag == TypeTag.BOOLEAN:
        _check(isinstance(value, bool), "expected boolean")
    elif expected.tag == TypeTag.VECTOR:
        _check(isinstance(value, Collection), "expected vector")
        if value:
            _check(isinstance(value[0], float), "expected vector of numbers")
    elif expected.tag == TypeTag.ENUM:
        # assumes literal/value enums
        _check(any(member.name == value for member in expected.fields), "expected enum member")
    elif expected.tag == TypeTag.STRUCT or expected.tag == TypeTag.FUNCTION:
        if expected.tag == TypeTag.FUNCTION and is_output and not expected.outputs:
            value = value or {}  # None is allowed for empty outputs
        if _check(isinstance(value, Mapping), "expected struct"):
            for f in expected.fields:
                if is_output is not None and bool(f.flags & TypeFlag.IsOutput) != is_output:
                    continue
                alt_name = to_pyidentifier(f.name)
                subvalue = value.get(f.name, value.get(alt_name))
                if subvalue is None:
                    _check(bool(f.flags & TypeFlag.IsNullable), "expected non-nullable value")
                else:
                    check_type(subvalue, f, eager_error=eager_error, on_invalid=on_invalid)
    elif expected.tag in (TypeTag.FILE,):
        _check(isinstance(value, RemoteObject), "expected remote object")
    elif expected.tag == TypeTag.UNION:
        for option in expected.fields:
            try:
                check_type(value, option)
                return
            except TypeError:
                pass
        _check(False, "expected one of the union types")
    elif expected.tag == TypeTag.NULL:
        _check(value is None, "expected null")
    elif expected.tag == TypeTag.ANY:
        pass
    elif expected.tag == TypeTag.LITERAL:
        _check(value == expected.value, "expected literal")
    else:
        raise RuntimeError(f"unexpected type {expected.tag}")

    if not eager_error and _suberrors:
        on_invalid(value, expected, suberrors=_suberrors)


def _map_v_noop(v, t):
    return v


def _map_k_noop(t):
    return t.name, t.name


def map_value(
    value: Any,
    type: TypeBase,
    map_v: Callable[[Any, TypeBase, bool], Any] = None,
    map_k: Callable[[Field], tuple[str, str]] = None,
    is_output: bool = None,
    ignore_array: bool = False,
    ignore_outer_map: bool = False,
):
    """Walks the value and reassembles with new keys and values."""
    map_v = map_v or _map_v_noop
    map_k = map_k or _map_k_noop
    # communicate via yield/send
    if type.flags & TypeFlag.IsArray and not ignore_array:
        if not isinstance(value, Collection):
            return value  # type error, ignore here
        return [map_value(item, type, map_v, map_k, ignore_array=True) for item in value]
    elif type.tag in PRIMITIVE_TYPES:
        return map_v(value=value, type=type, ignore_array=ignore_array)
    elif type.tag == TypeTag.ENUM:
        return map_v(value=value, type=type, ignore_array=ignore_array)
    elif type.tag not in (TypeTag.STRUCT, TypeTag.FUNCTION):
        raise TypeError(value, type, "expected struct-like")
    if not isinstance(value, Mapping):
        return value  # type error, ignore here
    mapped = {}
    for subtype in type.resolved_fields or type.fields:
        if subtype.flags & TypeFlag.IsUnionWith:  # unresolved union
            raise RuntimeError(f"unexpected union with {type}->{subtype}")
        if is_output is not None and bool(subtype.flags & TypeFlag.IsOutput) != is_output:
            continue
        source_k, target_k = map_k(subtype)
        if source_k not in value:
            continue  # ignore missing keys
        target_value = map_value(value[source_k], subtype, map_v, map_k)
        mapped[target_k] = target_value
    if not ignore_outer_map:
        mapped = map_v(value=mapped, type=type, ignore_array=ignore_array)
    return mapped


def map_unkey_enum(value: Any, type: TypeBase, *args, **kwargs):
    if type.tag == TypeTag.ENUM:
        return type[value].name
    return value


def map_rekey_enum(value: Any, type: TypeBase, *args, **kwargs):
    if type.tag == TypeTag.ENUM:
        return type[value].key
    return value


def unkey_value(
    value: Any,
    type: TypeBase,
    is_output: bool = None,
    ignore_array: bool = False,
    to_ident: bool = False,
) -> Any:
    """Replaces keys with actual values."""
    if to_ident:

        def map_k(t: Field):
            return t.typed_key, t.ident

    else:

        def map_k(t: Field):
            return t.typed_key, t.name

    return map_value(
        value,
        type,
        map_v=map_unkey_enum,
        map_k=map_k,
        is_output=is_output,
        ignore_array=ignore_array,
    )


def rekey_value(
    value: Any,
    type: TypeBase,
    is_output: bool = None,
    ignore_array: bool = False,
    from_ident: bool = False,
) -> Any:
    """Replaces names with keys."""
    if from_ident:

        def map_k(t: Field):
            return t.ident, t.typed_key

    else:

        def map_k(t: Field):
            return t.name, t.typed_key

    return map_value(
        value,
        type,
        map_v=map_rekey_enum,
        map_k=map_k,
        is_output=is_output,
        ignore_array=ignore_array,
    )


TYPENAME_SENTINEL = "__typename"  # :TypeSentinel
REMOTE_OBJECT_TYPENAME = "RemoteObject"
SECRET_TYPENAME = "Secret"
TypeSignature = typing.NamedTuple(
    "TypeSignature", [("tag", TypeTag), ("hint", Optional[TypeHint]), ("flags", TypeFlag)]
)


class TypeMapper:
    """
    Maps specific types (and values) into and from Python.
    Don't bother with lists and optional types here.
    """

    def to_py_type(self, type: TypeBase) -> type:
        raise NotImplementedError

    def maps_py_type(self, py_type: type) -> bool:
        raise NotImplementedError

    def from_py_type(self, py_type: type, type_map: dict[type, Any]) -> Type:
        raise NotImplementedError

    def to_py_value(self, type: TypeBase, value: Any) -> Any:
        return value

    def from_py_value(self, type: TypeBase, value: Any) -> Any:
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


def get_flat_mapper_by_type(type: TypeBase) -> TypeMapper:
    """
    Gets the most appropriate mapping for the given type.
    (flat because we ignore list and optional types).
    """
    # strip to only relevant flags for mapping
    stripped_flags = type.flags & TypeFlag.IsSecret
    exact_signature = TypeSignature(type.tag, type.hint, stripped_flags)
    mapping = type_mappers.get(exact_signature)
    if mapping is not None:
        return mapping
    # no exact match, try generic without hint
    stripped_signature = TypeSignature(type.tag, None, stripped_flags)
    mapping = type_mappers.get(stripped_signature)
    if mapping is not None:
        return mapping
    raise LookupError(f"no mapping found for {type}")


def get_flat_mapper_by_py_type(py_type: type) -> tuple[TypeMapper, type, TypeFlag]:
    """
    Gets the most appropriate mapping for the given Python type.
    (flat because we "ignore" list and optional types (inside the mapper)).
    """
    flags = TypeFlag.Zero
    # strip optional
    if typing.get_origin(py_type) is typing.Union:
        args = typing.get_args(py_type)
        if len(args) == 2 and args[1] == type(None):  # noqa: E721
            py_type = args[0]
            flags |= TypeFlag.IsNullable
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

    # get mapping
    for mapping in type_mappers.values():
        if mapping.maps_py_type(py_type):
            return mapping, py_type, flags
    raise LookupError(f"no mapping found for {py_type} ({flags}, type={type(py_type)})")


@dataclass(repr=False, slots=True)
class StaticTypeMapper(TypeMapper):
    py_type: type
    tag: TypeTag
    hint: Optional[TypeHint] = None

    def to_py_type(self, type: TypeBase) -> type:
        return self.py_type

    def maps_py_type(self, py_type: type) -> bool:
        return py_type == self.py_type

    def from_py_type(self, py_type: type, type_map: dict[type, Any]) -> Type:
        return Type(name=None, tag=self.tag, hint=self.hint)

    def to_py_value(self, type: TypeBase, value: Any) -> Any:
        return self.py_type(value)


@dataclass(repr=False, slots=True)
class NoopTypeMapper(TypeMapper):
    def to_py_type(self, type: TypeBase) -> type:
        return type

    def to_py_value(self, type: TypeBase, value: Any) -> Any:
        return value

    def from_py_value(self, type: TypeBase, value: Any) -> Any:
        return value


@dataclass(repr=False, slots=True)
class StringifyTypeMapping(StaticTypeMapper):
    def maps_py_type(self, py_type: type) -> bool:
        return self.py_type == py_type

    def to_py_value(self, type: TypeBase, value: Any) -> Any:
        return self.py_type(value)

    def from_py_value(self, type: TypeBase, value: Any) -> str:
        return str(value)


class IsoDtTypeMapping(StaticTypeMapper):
    HINT_BY_PY_TYPE = {
        date: TypeHint.DATE,
        datetime: TypeHint.DATETIME,
        time: TypeHint.TIME,
    }

    def to_py_value(self, type: TypeBase, value: Any) -> Any:
        return self.py_type.fromisoformat(value)

    def maps_py_type(self, py_type: type) -> bool:
        return inspect.isclass(py_type) and any(
            issubclass(py_type, t) for t in self.HINT_BY_PY_TYPE
        )

    def from_py_type(self, py_type: type, type_map: dict[type, Any]) -> Type:
        return Type(tag=TypeTag.STRING, hint=self.HINT_BY_PY_TYPE[py_type])

    def from_py_value(self, type: TypeBase, value: Any) -> str:
        return value.isoformat()


class EnumMapper(TypeMapper):
    def to_py_type(self, type: TypeBase) -> Any:
        members = {to_pyidentifier(child.name): child.name for child in type.fields}
        enum_name = type.name or "_anon_" + uuid4().hex
        return enum.StrEnum(enum_name, members)

    def maps_py_type(self, py_type: type) -> bool:
        return inspect.isclass(py_type) and issubclass(py_type, enum.StrEnum)

    def from_py_type(self, py_type: type, type_map: dict[type, Any]) -> TypeBase:
        assert issubclass(py_type, enum.StrEnum)
        type = Type(name=py_type.__name__, tag=TypeTag.ENUM)
        type_map[py_type] = type
        for py_member in py_type.__members__.values():
            member = Field(name=py_member.name, key=py_member.name, tag=TypeTag.LITERAL)
            type.fields.append(member)
        type._assign_oks()
        return type

    def to_py_value(self, type: TypeBase, value: Any) -> Any:
        return map_unkey_enum(value, type)

    def from_py_value(self, type: TypeBase, value: Any) -> Any:
        return map_rekey_enum(value, type)


class FileMapper(TypeMapper):
    def to_py_type(self, type: TypeBase) -> type:
        return RemoteObject

    def maps_py_type(self, py_type: type) -> bool:
        return py_type is RemoteObject

    def from_py_type(self, py_type: type, type_map: dict[type, Any]) -> TypeBase:
        return Type(name=None, tag=TypeTag.FILE)

    def to_py_value(self, type: TypeBase, value: Any) -> Any:
        return RemoteObject(
            id=UUID(value["id"]),
            name=value["name"],
            content_type=value["content_type"],
            content_length=value["content_length"],
            sha512=value["sha512"],
            status=RemoteObjectStatus[value["status"]],
        )

    def from_py_value(self, type: TypeBase, value: Any) -> Any:
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
    def to_py_type(self, type: TypeBase) -> Any:
        return Secret

    def maps_py_type(self, py_type: type) -> bool:
        return py_type is Secret

    def from_py_type(self, py_type: type, type_map: dict[type, Any]) -> TypeBase:
        return Type(name=None, tag=TypeTag.STRING, hint=TypeHint.SECRET, flags=TypeFlag.IsSecret)

    def to_py_value(self, type: TypeBase, value: Any) -> Any:
        return Secret(
            id=UUID(value["id"]),
            sha512=value["sha512"],
        )

    def from_py_value(self, type: TypeBase, value: Any) -> Any:
        return {
            TYPENAME_SENTINEL: SECRET_TYPENAME,
            "id": str(value.id),
            "sha512": value.sha512,
        }


class StructTypeMapper(TypeMapper):
    def to_py_type(self, type: TypeBase) -> typing.TypedDict:
        return typing.TypedDict(
            type.name,
            {member.ident: instantiate_py_type(member) for member in type.fields},
        )

    def maps_py_type(self, py_type: type) -> bool:
        return dataclasses.is_dataclass(py_type) or typing.is_typeddict(py_type)

    def from_py_type(self, py_type: type, type_map: dict[str, Any]) -> Type:
        type = Type(name=py_type.__name__, tag=TypeTag.STRUCT)
        type_map[py_type] = type
        if dataclasses.is_dataclass(py_type):
            for py_field in dataclasses.fields(py_type):
                field_ = field_from_py_field(py_field.type, py_field.name, type_map)
                type.fields.append(field_)
        elif typing.is_typeddict(py_type):
            for py_field_name, py_field in typing.get_type_hints(py_type).items():
                field_ = field_from_py_field(py_field, py_field_name, type_map)
                type.fields.append(field_)
        else:
            raise ValueError(f"unsupported struct type: {py_type}")
        type._assign_oks()
        return type

    def to_py_value(self, type: TypeBase, value: Any) -> Any:
        if not isinstance(type, Type):
            # may be a simple type node
            if isinstance(type.reference, Type):
                type = type.reference
            else:
                raise ValueError(f"struct type is not an instance: {type}")
        return type(**value)

    def from_py_value(self, type: TypeBase, value: Any) -> Any:
        return {TYPENAME_SENTINEL: type.key, **value}


class JsonTypeMapper(TypeMapper):
    def maps_py_type(self, py_type: type) -> bool:
        return py_type is Json or py_type is dict or typing.get_origin(py_type) is dict

    def from_py_type(self, py_type: type, type_map: dict[type, Any]) -> Type:
        return Type(name=None, tag=TypeTag.JSON)


class FunctionTypeMapper(TypeMapper):
    def maps_py_type(self, py_type: type) -> bool:
        return inspect.isfunction(py_type)

    def from_py_type(self, py_type: type, type_map: dict[type, Any]) -> Type:
        from bench.bench import wire

        type = Type(name=py_type.__name__, tag=TypeTag.FUNCTION)
        type_map[py_type] = type
        signature = inspect.signature(py_type)
        for py_param in signature.parameters.values():
            if py_param.name == "self" and (
                py_param.annotation is py_param.empty
                or py_param.annotation.__name__ is py_type.__name__
            ):
                continue
            param = field_from_py_field(py_param.annotation, py_param.name, type_map)
            type.fields.append(param)

        # output must be a struct, inline it with output flag
        if signature.return_annotation is inspect.Signature.empty:
            raise ValueError(f"missing return annotation for {py_type}")
        output = type_from_py_type(signature.return_annotation, None, type_map)
        if output.tag != TypeTag.STRUCT:
            raise ValueError(f"function output must be a struct: {py_type}")
        for field_ in output.fields:
            field_ = wire.unpack_node_flat(wire.pack_node_flat(field_), parent=type, session=None)
            field_.flags |= TypeFlag.IsOutput
            type.fields.append(field_)

        type._assign_oks()
        return type


def instantiate_py_type(node: TypeBase) -> type | Any | None:
    """Create the Python-native type for the given type node."""
    if node.tag == TypeTag.FUNCTION:
        return None  # functions don't have a pytype
    map = get_flat_mapper_by_type(node)
    py_type = map.to_py_type(node)
    if node.flags & TypeFlag.IsArray:
        return list[py_type]
    else:
        return py_type


_TYPE_MAP: dict[Any, Type] = {}


def type_from_py_type(
    py_type: type, name: Optional[str], type_map: dict[Any, Type] = None
) -> "Type":
    """
    Maps a python type to a Type (recursively).
    Nested types are read/written in the given type_map.
    Types are keyed by name since we have no way to associate keys over time.
    """
    type_map = type_map or _TYPE_MAP
    map, stripped, flags = get_flat_mapper_by_py_type(py_type)
    type = map.from_py_type(stripped, type_map)
    if name:
        type.name = name
    type.flags |= flags
    return type


def field_from_py_field(py_type: type | str, name: str, type_map: dict[Any, Type]) -> Field:
    if isinstance(py_type, str):
        # lookup by name in type_map
        type = first((t for k, t in type_map.items() if k.__name__ == py_type), None)
        if type is None:
            raise ValueError(f"unknown type name: {py_type}")
    else:
        type = type_from_py_type(py_type, name, type_map)
    if type.tag in (TypeTag.STRUCT, TypeTag.ENUM):
        # turn into reference
        return Field(name=name, key=name, tag=TypeTag.TYPE_REFERENCE, reference=type)
    else:
        return Field(name=name, key=name, tag=type.tag, hint=type.hint, flags=type.flags)


def instantiate_py_value_flat(value: Any, type: TypeBase, ignore_array: bool = False) -> Any:
    """Maps to the Python representation of the given value."""
    if value is None:  # skip null values
        return None  # type checking is done elsewhere
    # auto coerce lists to element and vice versa (like in frontend) :ArrayCoercion
    mapping = get_flat_mapper_by_type(type)
    try:
        if type.flags & TypeFlag.IsArray and not ignore_array:
            if not isinstance(value, list):
                value = [value]
            return [mapping.to_py_value(type, v) for v in value]
        else:
            if isinstance(value, list):
                value = value[0]
            return mapping.to_py_value(type, value)
    except (KeyError, ValueError, TypeError):
        logger.warning("instantiate_failed", exc_info=True, value=value, type=type)
        return value  # type checking is done elsewhere


def strip_py_value_flat(value: Any, type: TypeBase, *args, **kwargs) -> Any:
    """Maps back to the raw value from the Python representation."""
    # we don't auto-coerce here since that's only needed for external data
    if value is None:
        return None
    mapping = get_flat_mapper_by_type(type)
    return mapping.from_py_value(type, value)


# type tags
register_mapper(StaticTypeMapper(str, TypeTag.STRING), tags=[TypeTag.STRING])
register_mapper(StaticTypeMapper(Key, TypeTag.STRING, TypeHint.KEY), hints=[TypeHint.KEY])
register_mapper(StaticTypeMapper(float, TypeTag.NUMBER), tags=[TypeTag.NUMBER])
register_mapper(StaticTypeMapper(type(None), TypeTag.NULL), tags=[TypeTag.NULL])
register_mapper(StaticTypeMapper(bool, TypeTag.BOOLEAN), tags=[TypeTag.BOOLEAN])
register_mapper(StaticTypeMapper(Vector, TypeTag.VECTOR), tags=[TypeTag.VECTOR])
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
register_mapper(StaticTypeMapper(int, TypeTag.NUMBER, TypeHint.INTEGER), hints=[TypeHint.INTEGER])
# other
register_mapper(SecretTypeMapper(), tags=[TypeTag.STRING, TypeTag.NUMBER], flags=TypeFlag.IsSecret)
