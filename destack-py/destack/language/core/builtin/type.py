from typing import (
    TYPE_CHECKING,
    Any,
    Optional,
    Union,
    final,
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
)
from .property import builtin_property
from .struct import Struct, StructFrozen, builtin_struct

if TYPE_CHECKING:
    from destack.language import Value

# pyright: reportIncompatibleVariableOverride=false, reportIncompatibleMethodOverride=false

logger = get_logger(__name__)
tracer = get_tracer(__name__)


@builtin_struct(StructType.TYPE, frozen=True)
class Type(StructFrozen):
    """
    A basic Type in the type system. Types compose like a tree (with scalars at the leaves):
     - Scalar: a single value (self, Type.scalar_type)
     - List: a sequence of homogeneous values (Type.value_type)
     - Tuple: a sequence of heterogeneous values (Type.element_types)
     - Map: a mapping of homogenous keys to homogeneous values (Type.key_type->Type.value_type)
     - Literal: a constant value (self, Type.literal_value)
     - Union: a tagged union of heterogeneous values (Type.union_types)
    """

    # cardinality
    cardinality: TypeCardinality = builtin_property(
        110,
        is_repr=True,
        description="Cardinality of this Type.",
    )
    key_type: Optional["Type"] = builtin_property(
        111,
        is_repr=True,
        description="Key type of this Type (if it's a map).",
    )
    value_type: Optional["Type"] = builtin_property(
        112,
        is_repr=True,
        description="Value type of this Type (list, map).",
    )
    element_types: list["Type"] | None = builtin_property(
        113,
        is_repr=True,
        description="Element types of this Type (tuple).",
    )
    # dimensions: list[UInt32] | None = builtin_property(
    #     114,
    #     is_repr=True,
    #     description="Dimensions of this Type (ndarray).",
    # )
    is_required: bool = builtin_property(119, default=True)

    # scalar
    scalar_type: Optional[ScalarType] = builtin_property(
        120,
        is_repr=True,
        description="Scalar value type of this Type (primitive, enum, node, struct, etc..).",
    )
    primitive_type: Optional[PrimitiveType] = builtin_property(
        121,
        is_repr=True,
        description="Primitive type of this Type (if it's a primitive scalar).",
    )
    enum_type: Optional[EnumType] = builtin_property(
        122,
        is_repr=True,
        description="Enum type of this Type (if it's an enum scalar).",
    )
    node_types: list[NodeType] | None = builtin_property(
        123,
        is_repr=True,
        description="Node types of this Type (if it's a node reference or node value scalar).",
    )
    struct_type: Optional[StructType] = builtin_property(
        124,
        is_repr=True,
        description="Struct type of this Type (if it's a struct scalar).",
    )
    # literal_value: Optional["Value"] = builtin_property(
    #     125,
    #     is_repr=True,
    #     description="Literal value of this Type (if it's a literal scalar).",
    # )
    # union_types: list["Type"] | None = builtin_property(
    #     126,
    #     is_repr=True,
    #     description="Union types of this Type (if it's a union scalar).",
    # )

    @classmethod
    def infer(cls, value_or_type: Any, node_as_value: bool = False) -> "Type":
        """
        Infer the Type of a value or class.
        For values, we try to infer the most specific Type that can represent the value.
        For annotations, we defer to parse_type_annotation.
        """
        from ..builtin import Node, NodeReference, parse_type_annotation

        #
        # Values
        #

        # scalar values
        if value_or_type is None:
            return Type(
                cardinality=TypeCardinality.SCALAR,
                scalar_type=ScalarType.PRIMITIVE,
                primitive_type=PrimitiveType.NONE,
            )
        elif isinstance(value_or_type, NodeReference):
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
        elif isinstance(value_or_type, PRIMITIVE_PY_TYPES):
            return Type(
                cardinality=TypeCardinality.SCALAR,
                scalar_type=ScalarType.PRIMITIVE,
                primitive_type=PRIMITIVE_TYPE_BY_ANNOTATION[type(value_or_type)],
            )

        # collections
        if isinstance(value_or_type, tuple):
            # tuples are heterogeneous
            element_types = [
                CheckedType.infer(elem, node_as_value=node_as_value) for elem in value_or_type
            ]
            return Type(
                cardinality=TypeCardinality.TUPLE,
                element_types=element_types,  # type: ignore
            )
        elif isinstance(value_or_type, list):
            assert value_or_type, f"cannot infer type of empty list: {value_or_type!r}"
            # lists are homogeneous - infer from first element
            element_type = CheckedType.infer(value_or_type[0], node_as_value=node_as_value)
            return Type(
                cardinality=TypeCardinality.LIST,
                value_type=element_type,
            )
        elif isinstance(value_or_type, dict):
            assert value_or_type, f"cannot infer type of empty dict: {value_or_type!r}"
            sample_key = next(iter(value_or_type))
            sample_value = value_or_type[sample_key]
            key_type = CheckedType.infer(sample_key, node_as_value=node_as_value)
            value_type = CheckedType.infer(sample_value, node_as_value=node_as_value)
            return Type(
                cardinality=TypeCardinality.MAP,
                key_type=key_type,
                value_type=value_type,
            )

        #
        # Annotations
        #

        # parse as annotation
        type_decl = parse_type_annotation(value_or_type, is_builtin_member=False)
        return type_decl.to_type()


@builtin_struct(StructType.STRING_CONSTRAINT, frozen=True)
class StringConstraint(StructFrozen):
    """The constraint of a string."""

    regex: Optional[str] = builtin_property(41)
    starts_with: Optional[str] = builtin_property(42)
    ends_with: Optional[str] = builtin_property(43)


SLUG_REGEX_CHAR = r"a-z0-9-"
SLUG_REGEX = rf"^[{SLUG_REGEX_CHAR}]{{3,}}$"
EMAIL_REGEX = r"^[a-zA-Z0-9_.+\-]+@[a-zA-Z0-9\-]+\.[a-zA-Z0-9\-.]+$"
URL_REGEX = r"^(?:[a-z]+:\/\/)?[\w.-]+\.[a-z]{2,}(?:\/\S*)?$"
PHONE_NUMBER_REGEX = r"^\+?(\d{1,3})?[-.\s]?(\(?\d{1,4}\)?)?[-.\s]?\d{1,4}[-.\s]?\d{1,9}$"


@builtin_struct(
    StructType.NUMBER_CONSTRAINT,
    frozen=True,
    is_final=True,
)
@final
class NumberConstraint(StructFrozen):
    """The constraint of a number."""

    min_value: Optional[Float32] = builtin_property(41)
    max_value: Optional[Float32] = builtin_property(42)
    step_value: Optional[Float32] = builtin_property(43)


@builtin_struct(
    StructType.COLLECTION_CONSTRAINT,
    frozen=True,
    is_final=True,
)
@final
class CollectionConstraint(StructFrozen):
    """The constraint of a collection."""

    min_length: Optional[UInt32] = builtin_property(41)
    max_length: Optional[UInt32] = builtin_property(42)


TypeConstraint = Union[NumberConstraint, StringConstraint, CollectionConstraint]


@builtin_struct(StructType.CHECKED_TYPE, frozen=True)
class CheckedType(Type):
    """
    A full Type in the type system.
    Extends Type with defaults, constraints, and supporting flags.
    """

    # default
    default_value: Optional["Value"] = builtin_property(150)
    default_factory: Optional[ValueFactory] = builtin_property(151)

    # constraints
    collection_constraint: Optional["CollectionConstraint"] = builtin_property(160)
    string_constraint: Optional["StringConstraint"] = builtin_property(161)
    number_constraint: Optional["NumberConstraint"] = builtin_property(162)

    # flags
    # ...
