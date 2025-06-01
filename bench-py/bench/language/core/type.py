import types
import typing
from typing import (
    TYPE_CHECKING,
    Any,
    Optional,
    Union,
)

import structlog
from fastuuid import UUID
from opentelemetry import trace

from bench.language.registry import ENUM_TYPE_BY_CLASS

from .const import (
    PRIMITIVE_PY_TYPES,
    PRIMITIVE_TYPE_BY_PY_TYPE,
    BuiltinEnum,
    EnumType,
    NodeType,
    PrimitiveType,
    StructType,
    TraitType,
    enum_,
)
from .object import BuiltinObjectMutable, object_
from .property import property_
from .struct import StructBase, StructMutable, struct_

if TYPE_CHECKING:
    from bench.language import CustomNodeDefinition, Node, NodeReference, Value

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
    cardinality: TypeCardinality = property_(40, default=TypeCardinality.SCALAR, is_repr=True)
    scalar_type: ScalarType = property_(41, is_repr=True)
    primitive_type: Optional[PrimitiveType] = property_(42, is_repr=True)
    enum_type: Optional[EnumType] = property_(43, is_repr=True)
    node_type: Optional[NodeType] = property_(44, is_repr=True)
    node_definition: Optional["CustomNodeDefinition"] = property_(45, is_repr=True)
    struct_type: Optional[StructType] = property_(46, is_repr=True)
    base_type: Optional["Node"] = property_(47, is_repr=True)
    key_type: Optional["Type"] = property_(48, is_repr=True)  # for maps
    if TYPE_CHECKING:
        base_id: Optional[UUID] = None
        base_ptr: Optional["NodeReference"] = None

    # meta
    is_required: bool = property_(50, default=False)
    is_variable: bool = property_(51, default=False)
    # is_external
    default: Optional["Value"] = property_(55)
    default_factory: Optional[DefaultFactory] = property_(56)

    # constraints
    collection_constraint: Optional["CollectionConstraint"] = property_(60)
    string_constraint: Optional["StringConstraint"] = property_(61)
    number_constraint: Optional["NumberConstraint"] = property_(62)
    node_constraint: Optional["NodeConstraint"] = property_(63)


@struct_(StructType.TYPE)
class Type(StructMutable, TypeBase):  # NOTE: it would be nice to have Type be frozen..
    """A Type in the type system."""

    pass


def to_type(value_or_type: Any) -> "Type":
    """
    Guess the Type of a value or class.
    For values, we infer the most specific Type that can represent the value.
    """
    from bench.language import Node, NodeReference

    if value_or_type is None:
        raise ValueError("None is not a valid Type")

    # scalar values
    if isinstance(value_or_type, NodeReference):
        return Type(
            cardinality=TypeCardinality.SCALAR,
            scalar_type=ScalarType.NODE,
            node_type=value_or_type.node_type,
        )
    elif isinstance(value_or_type, Node):
        return Type(
            cardinality=TypeCardinality.SCALAR,
            scalar_type=ScalarType.NODE,
            node_type=value_or_type.metatype,
        )
    elif isinstance(value_or_type, StructBase):
        return Type(
            cardinality=TypeCardinality.SCALAR,
            scalar_type=ScalarType.STRUCT,
            struct_type=value_or_type.metatype,
        )
    elif isinstance(value_or_type, BuiltinEnum):
        return Type(
            cardinality=TypeCardinality.SCALAR,
            scalar_type=ScalarType.ENUM,
            enum_type=ENUM_TYPE_BY_CLASS[type(value_or_type)],
        )
    elif isinstance(value_or_type, type) and (
        primitive_type := PRIMITIVE_TYPE_BY_PY_TYPE.get(value_or_type)
    ):
        return Type(
            cardinality=TypeCardinality.SCALAR,
            scalar_type=ScalarType.PRIMITIVE,
            primitive_type=primitive_type,
        )
    elif isinstance(value_or_type, PRIMITIVE_PY_TYPES):
        return Type(
            cardinality=TypeCardinality.SCALAR,
            scalar_type=ScalarType.PRIMITIVE,
            primitive_type=PRIMITIVE_TYPE_BY_PY_TYPE[type(value_or_type)],
        )

    # collections
    if isinstance(value_or_type, (list, tuple)):
        assert value_or_type, f"cannot infer type of empty sequence: {value_or_type!r}"
        element_type = to_type(value_or_type[0])
        return Type(
            cardinality=TypeCardinality.LIST,
            scalar_type=element_type.scalar_type,
            primitive_type=element_type.primitive_type,
            enum_type=element_type.enum_type,
            node_type=element_type.node_type,
            struct_type=element_type.struct_type,
            node_constraint=element_type.node_constraint,
        )
    elif isinstance(value_or_type, dict):
        assert value_or_type, f"cannot infer type of empty dict: {value_or_type!r}"
        sample_key = next(iter(value_or_type))
        sample_value = value_or_type[sample_key]
        key_type = to_type(sample_key)
        value_type = to_type(sample_value)
        return Type(
            cardinality=TypeCardinality.MAP,
            scalar_type=value_type.scalar_type,
            primitive_type=value_type.primitive_type,
            enum_type=value_type.enum_type,
            node_type=value_type.node_type,
            struct_type=value_type.struct_type,
            node_constraint=value_type.node_constraint,
            key_type=key_type,
        )

    # handle type annotations
    origin = typing.get_origin(value_or_type)
    if origin in (list, tuple):
        element_type_annotation = typing.get_args(value_or_type)[0]
        element_type = to_type(element_type_annotation)
        return Type(
            cardinality=TypeCardinality.LIST,
            scalar_type=element_type.scalar_type,
            primitive_type=element_type.primitive_type,
            enum_type=element_type.enum_type,
            node_type=element_type.node_type,
            struct_type=element_type.struct_type,
            node_constraint=element_type.node_constraint,
        )
    elif origin is dict:
        key_type_annotation, value_type_annotation = typing.get_args(value_or_type)
        key_type = to_type(key_type_annotation)
        value_type = to_type(value_type_annotation)
        return Type(
            cardinality=TypeCardinality.MAP,
            scalar_type=value_type.scalar_type,
            primitive_type=value_type.primitive_type,
            enum_type=value_type.enum_type,
            node_type=value_type.node_type,
            struct_type=value_type.struct_type,
            node_constraint=value_type.node_constraint,
            key_type=key_type,
        )
    elif origin in (typing.Union, types.UnionType):
        union_args = typing.get_args(value_or_type)
        non_none_types = tuple(t for t in union_args if t is not type(None))
        assert len(non_none_types) > 0, f"empty union: {value_or_type!r}"

        # check if all types are Node types
        node_types = []
        for arg_type in non_none_types:
            if (
                isinstance(arg_type, type)
                and issubclass(arg_type, Node)
                and (metatype := getattr(arg_type, "metatype", None))
            ):
                node_types.append(metatype)
            else:
                # not all are Node types, fall back to first type
                return to_type(non_none_types[0])

        if node_types:
            return Type(
                cardinality=TypeCardinality.SCALAR,
                scalar_type=ScalarType.NODE,
                node_constraint=NodeConstraint(node_types=node_types),
            )

    # type classes
    if isinstance(value_or_type, type):
        if issubclass(value_or_type, BuiltinEnum):
            return Type(
                cardinality=TypeCardinality.SCALAR,
                scalar_type=ScalarType.ENUM,
                enum_type=ENUM_TYPE_BY_CLASS[value_or_type],
            )
        elif issubclass(value_or_type, Node):
            return Type(
                cardinality=TypeCardinality.SCALAR,
                scalar_type=ScalarType.NODE,
                node_type=getattr(value_or_type, "metatype", None),
            )
        elif issubclass(value_or_type, StructBase):
            return Type(
                cardinality=TypeCardinality.SCALAR,
                scalar_type=ScalarType.STRUCT,
                struct_type=value_or_type.metatype,
            )

    raise ValueError(f"cannot infer type of {value_or_type!r}")
