from typing import (
    TYPE_CHECKING,
    Any,
    Optional,
    Sequence,
    Union,
    cast,
)

import structlog
from fastuuid import UUID
from opentelemetry import trace

from bench.language.registry import BENCH_TYPE_BY_CLASS

from .const import (
    PRIMITIVE_TYPE_BY_PY_TYPE,
    BuiltinEnum,
    EnumType,
    NodeType,
    PrimitiveType,
    PrimitiveValue,
    StructType,
    Trait,
    enum_,
    is_enum_type,
    is_node_type,
    is_struct_type,
)
from .object import BuiltinObject, get_tk_b64_from_ck, object_
from .property import p_internal, p_regular
from .struct import Struct, struct_

if TYPE_CHECKING:
    from bench.language import Field, FileType, Node, NodeReference, Value

# pyright: reportIncompatibleVariableOverride=false, reportIncompatibleMethodOverride=false

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


@enum_(EnumType.TYPE_CARDINALITY)
class TypeCardinality(BuiltinEnum):
    """The 'kind' of a Type."""

    SCALAR = 1
    LIST = 2
    MAP = 3
    # OPTION = 4
    # LITERAL = 5
    # UNION = 6


@enum_(EnumType.SCALAR_TYPE)
class ScalarType(BuiltinEnum):
    """The type of a scalar."""

    PRIMITIVE = 1
    ENUM = 2
    NODE = 3
    STRUCT = 4


@enum_(EnumType.STRING_FORMAT)
class StringFormat(BuiltinEnum):
    """The format of a string."""

    NAME = 1
    SLUG = 2
    EMAIL = 3
    UUID = 10
    URL = 11
    EMOJI = 12
    MIME = 13
    BASE64 = 20


@enum_(EnumType.NUMBER_FORMAT)
class NumberFormat(BuiltinEnum):
    """The format of a number."""

    INTEGER = 1
    FLOAT = 2


@struct_(StructType.STRING_CONSTRAINT)
class StringConstraint(Struct):
    """The constraint of a string."""

    format: Optional[StringFormat] = p_regular(40)
    regex: Optional[str] = p_regular(41)
    starts_with: Optional[str] = p_regular(42)
    ends_with: Optional[str] = p_regular(43)


SLUG_REGEX_CHAR = r"a-z0-9-"
SLUG_REGEX = rf"^[{SLUG_REGEX_CHAR}]{{3,}}$"
EMAIL_REGEX = r"^[a-zA-Z0-9_.+\-]+@[a-zA-Z0-9\-]+\.[a-zA-Z0-9\-.]+$"
URL_REGEX = r"^(?:[a-z]+:\/\/)?[\w.-]+\.[a-z]{2,}(?:\/\S*)?$"
PHONE_NUMBER_REGEX = r"^\+?(\d{1,3})?[-.\s]?(\(?\d{1,4}\)?)?[-.\s]?\d{1,4}[-.\s]?\d{1,9}$"


@struct_(StructType.NUMBER_CONSTRAINT)
class NumberConstraint(Struct):
    """The constraint of a number."""

    format: Optional[NumberFormat] = p_regular(40)
    min_value: Optional[float] = p_regular(41)
    max_value: Optional[float] = p_regular(42)
    step_value: Optional[float] = p_regular(43)
    precision: Optional[int] = p_regular(44)  # for decimals
    scale: Optional[int] = p_regular(45)  # for decimals


@struct_(StructType.COLLECTION_CONSTRAINT)
class CollectionConstraint(Struct):
    """The constraint of a collection."""

    min_length: Optional[int] = p_regular(41)
    max_length: Optional[int] = p_regular(42)


@struct_(StructType.NODE_CONSTRAINT)
class NodeConstraint(Struct):
    """The constraint of a node."""

    node_types: list["NodeType"] = p_regular(41)
    node_traits: list["Trait"] = p_regular(42)
    # page/thread/base/bench, ...


Format = Union[NumberFormat, StringFormat]
Constraint = Union[NumberConstraint, NodeConstraint, StringConstraint, CollectionConstraint]
type Json = Any


@object_()
class TypeBase(BuiltinObject):
    """
    A Type describes the shape of a value.
    For lists and maps, the scalar type describes the element/value type.
    """

    cardinality: TypeCardinality = p_internal(40)

    # scalar
    scalar_type: ScalarType = p_internal(41)
    primitive_type: Optional[PrimitiveType] = p_regular(42)
    enum_type: Optional[EnumType] = p_regular(43)
    node_type: Optional[NodeType] = p_regular(44)
    struct_type: Optional[StructType] = p_regular(45)
    default: Optional["Value"] = p_regular(46)
    is_required: bool = p_regular(47, default=False)
    is_variable: bool = p_regular(48, default=False)

    # collection
    base_type: Optional["Node"] = p_regular(50)
    key_type: Optional["Type"] = p_regular(51)  # for maps
    if TYPE_CHECKING:
        base_type_id: Optional[UUID] = None
        base_type_ptr: Optional["NodeReference"] = None

    # constraints
    collection_constraint: Optional["CollectionConstraint"] = p_regular(60)
    string_constraint: Optional["StringConstraint"] = p_regular(61)
    number_constraint: Optional["NumberConstraint"] = p_regular(62)
    node_constraint: Optional["NodeConstraint"] = p_regular(63)

    def morph_to(
        self,
        typ: "TypeIn",
        constraint: Constraint | None = None,
        is_required: bool = False,
    ):
        """Change this type to another type."""
        typ = to_type(typ, constraint=constraint, is_required=is_required)
        for prop in TypeBase.__declared_properties__.values():
            new_typ_value = getattr(typ, prop.name)
            old_typ_value = getattr(self, prop.name)
            if new_typ_value != old_typ_value:
                setattr(self, prop.name, new_typ_value)

    @property
    def identity_key(self) -> str:
        """The identity of this type for packing."""
        return encode_type_identity(self)

    @property
    def _base_fields(self) -> Sequence["Field"]:
        if (base_type := self.base_type) is not None:
            return cast(
                Sequence["Field"], base_type._graph.get_descendants(base_type, NodeType.FIELD)
            )
        else:
            return ()

    @property
    def _fields(self) -> Sequence["Field"]:
        return self._base_fields


def encode_type_identity(typ: "TypeBase") -> str:
    """
    Encodes the type identity into a key for storage & implicit typing.
    Format is <kind>[id] (with id encoded as base64).
    :TypeInfoEncoding
    """

    raise NotImplementedError


def decode_type_identity(key: str) -> "TypeBase":
    """Decodes the type-related info back from the identity key. See encode. :TypeInfoEncoding"""
    raise NotImplementedError


def encode_storage_key(field: "Field") -> str:
    """Gets the key used to identify values of this field in storage. :FieldStorageKey"""
    return f"{get_tk_b64_from_ck(field.ck)}{field.identity_key}"


@struct_(StructType.TYPE)
class Type(Struct, TypeBase):
    """A Type in the type system."""

    # redirect so we get TypeBase.__content_str__ (not Struct.__content_str__)
    __content_str__ = TypeBase.__content_str__  # type: ignore


#
# Convenience to type utilities
#

TypeIn = Union[
    "TypeBase",
    "Node",
    "BuiltinEnum",
    "PrimitiveType",
    "FileType",
    type["Struct"],
    type["Node"],
    type[PrimitiveValue],
    type[BuiltinEnum],
]


def to_type_scalar(type_in: TypeIn) -> "Type":
    """Converts a type-like object to a Type."""
    from bench.language import (
        Action,
        Agent,
        Block,
        Flow,
        FlowEdge,
        Schema,
        Table,
    )

    if isinstance(type_in, Block) and (node := type_in.node) is not None:
        type_in = cast(TypeIn, node)  # unpack inner node automatically

    if isinstance(type_in, TypeBase):
        return cast("Type", type_in)
    elif isinstance(type_in, (Schema, Flow, Action, FlowEdge, Table, Agent)):
        raise NotImplementedError
        type_scalar = type_in.to_type_maybe()
        if type_scalar is not None:
            assert isinstance(type_scalar, Type), f"expected Type, got {type_scalar!r}"
            return type_scalar
    elif isinstance(type_in, PrimitiveType):
        return Type(
            cardinality=TypeCardinality.SCALAR,
            scalar_type=ScalarType.PRIMITIVE,
            primitive_type=type_in,
        )
    elif isinstance(type_in, (NodeType, StructType, EnumType)):
        if is_node_type(type_in):
            return Type(
                cardinality=TypeCardinality.SCALAR,
                scalar_type=ScalarType.NODE,
                node_type=type_in,
            )
        elif is_struct_type(type_in):
            return Type(
                cardinality=TypeCardinality.SCALAR,
                scalar_type=ScalarType.STRUCT,
                struct_type=type_in,
            )
        elif is_enum_type(type_in):
            return Type(
                cardinality=TypeCardinality.SCALAR,
                scalar_type=ScalarType.ENUM,
                enum_type=type_in,
            )
    elif isinstance(type_in, type):
        primitive_type = PRIMITIVE_TYPE_BY_PY_TYPE.get(type_in)
        if primitive_type:
            return Type(
                cardinality=TypeCardinality.SCALAR,
                scalar_type=ScalarType.PRIMITIVE,
                primitive_type=primitive_type,
            )
        bench_type = BENCH_TYPE_BY_CLASS.get(cast(Any, type_in))
        if bench_type is not None:
            if is_node_type(bench_type):
                return Type(
                    cardinality=TypeCardinality.SCALAR,
                    scalar_type=ScalarType.NODE,
                    node_type=bench_type,
                )
            elif is_struct_type(bench_type):
                return Type(
                    cardinality=TypeCardinality.SCALAR,
                    scalar_type=ScalarType.STRUCT,
                    struct_type=bench_type,
                )
            elif is_enum_type(bench_type):
                return Type(
                    cardinality=TypeCardinality.SCALAR,
                    scalar_type=ScalarType.ENUM,
                    enum_type=bench_type,
                )
            return Type(cardinality=TypeCardinality.SCALAR, scalar_type=ScalarType.NODE)

    raise ValueError(f"unsupported type {type_in!r}")


def to_type(
    type_in: TypeIn,
    *,
    constraint: Constraint | None = None,
    is_required: bool = False,
    is_list: bool = False,
) -> Type:
    """Converts a TypeIn into a Type."""
    raise NotImplementedError


def reverse_type_scalar(typ: TypeBase) -> TypeIn | None:
    """
    Reverses a Type into a TypeIn as closely as possible.
    Does not consider non-scalar properties (is_list, is_required, etc.)
    """
    raise NotImplementedError
