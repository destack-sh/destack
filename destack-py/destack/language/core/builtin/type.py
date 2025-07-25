import types
import typing
from typing import (
    TYPE_CHECKING,
    Any,
    Optional,
    TypeAliasType,
    Union,
)

from destack.language.registry import ENUM_TYPE_BY_CLASS
from destack.utils.log import get_logger
from destack.utils.telemetry import get_tracer

from .builtin import EnumType, NodeType, StructType
from .common import (
    PRIMITIVE_PY_TYPES,
    PRIMITIVE_TYPE_BY_ANNOTATION,
    Enum,
    Float32,
    PrimitiveType,
    ScalarType,
    TypeCardinality,
    UInt32,
    ValueFactory,
    builtin_enum,
)
from .property import builtin_property
from .struct import Struct, StructFrozen, builtin_struct

if TYPE_CHECKING:
    from destack.language import Value

# pyright: reportIncompatibleVariableOverride=false, reportIncompatibleMethodOverride=false

logger = get_logger(__name__)
tracer = get_tracer(__name__)


@builtin_enum(EnumType.STRING_FORMAT)
class StringFormat(Enum):
    """The format of a string."""

    NAME = 1
    SLUG = 2
    EMAIL = 3
    UUID = 10
    URL = 11
    EMOJI = 12
    MIME = 13
    BASE64 = 20


@builtin_enum(EnumType.NUMBER_FORMAT)
class NumberFormat(Enum):
    """The format of a number."""

    PERCENTAGE = 1
    ANGLE = 2
    CURRENCY = 3


@builtin_struct(StructType.STRING_CONSTRAINT, frozen=True)
class StringConstraint(StructFrozen):
    """The constraint of a string."""

    format: Optional[StringFormat] = builtin_property(40)
    regex: Optional[str] = builtin_property(41)
    starts_with: Optional[str] = builtin_property(42)
    ends_with: Optional[str] = builtin_property(43)


SLUG_REGEX_CHAR = r"a-z0-9-"
SLUG_REGEX = rf"^[{SLUG_REGEX_CHAR}]{{3,}}$"
EMAIL_REGEX = r"^[a-zA-Z0-9_.+\-]+@[a-zA-Z0-9\-]+\.[a-zA-Z0-9\-.]+$"
URL_REGEX = r"^(?:[a-z]+:\/\/)?[\w.-]+\.[a-z]{2,}(?:\/\S*)?$"
PHONE_NUMBER_REGEX = r"^\+?(\d{1,3})?[-.\s]?(\(?\d{1,4}\)?)?[-.\s]?\d{1,4}[-.\s]?\d{1,9}$"


@builtin_struct(StructType.NUMBER_CONSTRAINT, frozen=True)
class NumberConstraint(StructFrozen):
    """The constraint of a number."""

    format: Optional[NumberFormat] = builtin_property(40)
    min_value: Optional[Float32] = builtin_property(41)
    max_value: Optional[Float32] = builtin_property(42)
    step_value: Optional[Float32] = builtin_property(43)


@builtin_struct(StructType.COLLECTION_CONSTRAINT, frozen=True)
class CollectionConstraint(StructFrozen):
    """The constraint of a collection."""

    min_length: Optional[UInt32] = builtin_property(41)
    max_length: Optional[UInt32] = builtin_property(42)


TypeConstraint = Union[NumberConstraint, StringConstraint, CollectionConstraint]


@builtin_struct(StructType.BASIC_TYPE, frozen=True)
class BasicType(StructFrozen):
    """A basic Type in the type system."""

    cardinality: TypeCardinality = builtin_property(
        110,
        default=TypeCardinality.SCALAR,
        is_repr=True,
        description="Cardinality of this Type (scalar, list, map, etc.)",
    )
    scalar_type: ScalarType = builtin_property(
        111,
        is_repr=True,
        description="Scalar value type of this Type (primitive, enum, node, struct, etc..).",
    )
    primitive_type: Optional[PrimitiveType] = builtin_property(
        112,
        is_repr=True,
        description="Primitive type of this Type (if it's a primitive value).",
    )
    enum_type: Optional[EnumType] = builtin_property(
        113,
        is_repr=True,
        description="Enum type of this Type (if it's an enum value).",
    )
    node_types: list[NodeType] | None = builtin_property(
        114,
        is_repr=True,
        description="Node types of this Type (if it's a node reference value).",
    )
    struct_type: Optional[StructType] = builtin_property(
        115,
        is_repr=True,
        description="Struct type of this Type (if it's a struct value).",
    )
    key_type: Optional["Type"] = builtin_property(
        117,
        is_repr=True,
        description="Key type of this Type (if it's a map value).",
    )
    literal_value: Optional["Value"] = builtin_property(
        120,
        is_repr=True,
        description="Value of this Type (if it's a literal value).",
    )


@builtin_struct(StructType.TYPE, frozen=True)
class Type(BasicType):
    """A full Type in the type system."""

    # default
    default_value: Optional["Value"] = builtin_property(150)
    default_factory: Optional[ValueFactory] = builtin_property(151)

    # constraints
    collection_constraint: Optional["CollectionConstraint"] = builtin_property(160)
    string_constraint: Optional["StringConstraint"] = builtin_property(161)
    number_constraint: Optional["NumberConstraint"] = builtin_property(162)

    # flags
    is_required: bool | None = builtin_property(170)


def to_type(value_or_type: Any, node_as_value: bool = False) -> "Type":
    """
    Guess the Type of a value or class.
    For values, we try to infer the most specific Type that can represent the value.
    """
    from destack.language import Node, NodeReference

    if value_or_type is None:
        raise ValueError("None is not a valid Type")

    # scalar values
    if isinstance(value_or_type, NodeReference):
        return Type(
            cardinality=TypeCardinality.SCALAR,
            scalar_type=ScalarType.NODE_REFERENCE,
            node_types=[value_or_type.type],
        )
    elif isinstance(value_or_type, Node):
        return Type(
            cardinality=TypeCardinality.SCALAR,
            scalar_type=ScalarType.NODE_VALUE if node_as_value else ScalarType.NODE_REFERENCE,
            node_types=[value_or_type.metatype],
        )
    elif isinstance(value_or_type, Struct):
        return Type(
            cardinality=TypeCardinality.SCALAR,
            scalar_type=ScalarType.STRUCT,
            struct_type=value_or_type.metatype,
        )
    elif isinstance(value_or_type, Enum):
        return Type(
            cardinality=TypeCardinality.SCALAR,
            scalar_type=ScalarType.ENUM,
            enum_type=ENUM_TYPE_BY_CLASS[type(value_or_type)],
        )
    elif isinstance(value_or_type, (type, TypeAliasType)) and (
        primitive_type := PRIMITIVE_TYPE_BY_ANNOTATION.get(value_or_type)
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
            primitive_type=PRIMITIVE_TYPE_BY_ANNOTATION[type(value_or_type)],
        )

    # collections
    if isinstance(value_or_type, (list, tuple)):
        assert value_or_type, f"cannot infer type of empty sequence: {value_or_type!r}"
        element_type = to_type(value_or_type[0])
        assert element_type.cardinality == TypeCardinality.SCALAR, (
            f"expected scalar inside list, got {element_type!r} for {value_or_type!r}"
        )
        return Type(
            cardinality=TypeCardinality.LIST,
            scalar_type=element_type.scalar_type,
            primitive_type=element_type.primitive_type,
            enum_type=element_type.enum_type,
            node_types=element_type.node_types,
            struct_type=element_type.struct_type,
        )
    elif isinstance(value_or_type, dict):
        assert value_or_type, f"cannot infer type of empty dict: {value_or_type!r}"
        sample_key = next(iter(value_or_type))
        sample_value = value_or_type[sample_key]
        key_type = to_type(sample_key)
        assert key_type.cardinality == TypeCardinality.SCALAR, (
            f"expected scalar inside dict, got {key_type!r} for {value_or_type!r}"
        )
        value_type = to_type(sample_value)
        assert value_type.cardinality in (TypeCardinality.SCALAR, TypeCardinality.LIST), (
            f"expected scalar or list inside dict, got {value_type!r} for {value_or_type!r}"
        )
        return Type(
            cardinality=TypeCardinality.MAP,
            scalar_type=value_type.scalar_type,
            primitive_type=value_type.primitive_type,
            enum_type=value_type.enum_type,
            node_types=value_type.node_types,
            struct_type=value_type.struct_type,
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
            node_types=element_type.node_types,
            struct_type=element_type.struct_type,
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
            node_types=value_type.node_types,
            struct_type=value_type.struct_type,
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
                scalar_type=ScalarType.NODE_REFERENCE,
                node_types=node_types,
            )

    # type classes
    if isinstance(value_or_type, type):
        if issubclass(value_or_type, Enum):
            return Type(
                cardinality=TypeCardinality.SCALAR,
                scalar_type=ScalarType.ENUM,
                enum_type=ENUM_TYPE_BY_CLASS[value_or_type],
            )
        elif issubclass(value_or_type, Node):
            node_type = getattr(value_or_type, "metatype", None)
            if not isinstance(node_type, NodeType):
                raise ValueError(f"cannot infer node type of {value_or_type!r}")
            return Type(
                cardinality=TypeCardinality.SCALAR,
                scalar_type=ScalarType.NODE_REFERENCE,
                node_types=[node_type],
            )
        elif issubclass(value_or_type, Struct):
            return Type(
                cardinality=TypeCardinality.SCALAR,
                scalar_type=ScalarType.STRUCT,
                struct_type=value_or_type.metatype,
            )

    raise ValueError(f"cannot infer type of {value_or_type!r}")
