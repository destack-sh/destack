from typing import (
    TYPE_CHECKING,
    Any,
    Optional,
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
    TraitType,
    enum_,
)
from .object import BuiltinObjectMutable, object_
from .property import property_
from .struct import StructBase, StructMutable, struct_

if TYPE_CHECKING:
    from bench.language import Field, FileType, Node, NodeReference, Table, Value

# pyright: reportIncompatibleVariableOverride=false, reportIncompatibleMethodOverride=false

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


@enum_(EnumType.TYPE_CARDINALITY)
class TypeCardinality(BuiltinEnum):
    """The 'kind' of a Type."""

    SCALAR = 1
    LIST = 2
    # SET?
    MAP = 4
    # OPTION = 5
    # LITERAL = 6
    # UNION = 7


@enum_(EnumType.SCALAR_TYPE)
class ScalarType(BuiltinEnum):
    """The type of a scalar."""

    PRIMITIVE = 1
    ENUM = 2
    NODE = 3
    STRUCT = 4


@enum_(EnumType.DEFAULT_FACTORY)
class DefaultFactory(BuiltinEnum):
    """The factory to use for default values."""

    UUID = 1
    NOW = 2
    REGION = 3


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

    PERCENTAGE = 1
    ANGLE = 2
    CURRENCY = 3


@struct_(StructType.STRING_CONSTRAINT)
class StringConstraint(StructMutable):
    """The constraint of a string."""

    format: Optional[StringFormat] = property_(40)
    regex: Optional[str] = property_(41)
    starts_with: Optional[str] = property_(42)
    ends_with: Optional[str] = property_(43)


SLUG_REGEX_CHAR = r"a-z0-9-"
SLUG_REGEX = rf"^[{SLUG_REGEX_CHAR}]{{3,}}$"
EMAIL_REGEX = r"^[a-zA-Z0-9_.+\-]+@[a-zA-Z0-9\-]+\.[a-zA-Z0-9\-.]+$"
URL_REGEX = r"^(?:[a-z]+:\/\/)?[\w.-]+\.[a-z]{2,}(?:\/\S*)?$"
PHONE_NUMBER_REGEX = r"^\+?(\d{1,3})?[-.\s]?(\(?\d{1,4}\)?)?[-.\s]?\d{1,4}[-.\s]?\d{1,9}$"


@struct_(StructType.NUMBER_CONSTRAINT)
class NumberConstraint(StructMutable):
    """The constraint of a number."""

    format: Optional[NumberFormat] = property_(40)
    min_value: Optional[float] = property_(41)
    max_value: Optional[float] = property_(42)
    step_value: Optional[float] = property_(43)
    precision: Optional[int] = property_(44)  # for decimals
    scale: Optional[int] = property_(45)  # for decimals


@struct_(StructType.COLLECTION_CONSTRAINT)
class CollectionConstraint(StructMutable):
    """The constraint of a collection."""

    min_length: Optional[int] = property_(41)
    max_length: Optional[int] = property_(42)


@struct_(StructType.NODE_CONSTRAINT)
class NodeConstraint(StructMutable):
    """The constraint of a node."""

    node_types: list["NodeType"] = property_(41)
    node_traits: list["TraitType"] = property_(42)
    # page/thread/base/bench, ...


Format = Union[NumberFormat, StringFormat]
Constraint = Union[NumberConstraint, NodeConstraint, StringConstraint, CollectionConstraint]
type Json = Any


@object_()
class TypeBase(BuiltinObjectMutable):
    """
    A Type describes the shape of a value.
    For lists and maps, the scalar type describes the element/value type.
    """

    # scalar
    cardinality: TypeCardinality = property_(40)
    scalar_type: ScalarType = property_(41)
    primitive_type: Optional[PrimitiveType] = property_(42)
    enum_type: Optional[EnumType] = property_(43)
    node_type: Optional[NodeType] = property_(44)
    table: Optional["Table"] = property_(45)  # for nodes
    struct_type: Optional[StructType] = property_(46)
    base_type: Optional["Node"] = property_(47)
    key_type: Optional["Type"] = property_(48)  # for maps
    if TYPE_CHECKING:
        schema_id: Optional[UUID] = None
        schema_ptr: Optional["NodeReference"] = None
        table_id: Optional[UUID] = None
        table_ptr: Optional["NodeReference"] = None

    # meta
    is_required: bool = property_(50, default=False)
    is_variable: bool = property_(51, default=False)
    is_external: bool = property_(
        52,
        default=False,
        description="Whether this type is defined outside of Bench.",
    )
    # external_id, external_key, ...?
    default: Optional["Value"] = property_(55)
    default_factory: Optional[DefaultFactory] = property_(56)

    # constraints
    collection_constraint: Optional["CollectionConstraint"] = property_(60)
    string_constraint: Optional["StringConstraint"] = property_(61)
    number_constraint: Optional["NumberConstraint"] = property_(62)
    node_constraint: Optional["NodeConstraint"] = property_(63)


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
    raise NotImplementedError


@struct_(StructType.TYPE)
class Type(StructMutable, TypeBase):
    """A Type in the type system."""

    pass


#
# Convenience to type utilities
#

TypeIn = Union[
    "TypeBase",
    "Node",
    "BuiltinEnum",
    "PrimitiveType",
    "FileType",
    type["StructBase"],
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
        if isinstance(type_in, NodeType):
            return Type(
                cardinality=TypeCardinality.SCALAR,
                scalar_type=ScalarType.NODE,
                node_type=type_in,
            )
        elif isinstance(type_in, StructType):
            return Type(
                cardinality=TypeCardinality.SCALAR,
                scalar_type=ScalarType.STRUCT,
                struct_type=type_in,
            )
        elif isinstance(type_in, EnumType):
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
            if isinstance(bench_type, NodeType):
                return Type(
                    cardinality=TypeCardinality.SCALAR,
                    scalar_type=ScalarType.NODE,
                    node_type=bench_type,
                )
            elif isinstance(bench_type, StructType):
                return Type(
                    cardinality=TypeCardinality.SCALAR,
                    scalar_type=ScalarType.STRUCT,
                    struct_type=bench_type,
                )
            elif isinstance(bench_type, EnumType):
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
