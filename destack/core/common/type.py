from typing import TYPE_CHECKING, Any, Optional, Union, assert_never, final

from destack.registry import (
    ENUM_CLASS_BY_TYPE,
    ENUM_TYPE_BY_CLASS,
    HANDLE_CLASS_BY_TYPE,
    NODE_CLASS_BY_TYPE,
    STRUCT_CLASS_BY_TYPE,
)

from ..builtin import (
    PRIMITIVE_PY_ANNOTATION_BY_TYPE,
    PRIMITIVE_TYPE_BY_ANNOTATION,
    Enum,
    EnumType,
    Float32,
    HandleType,
    NodeType,
    OptionDeclaration,
    PrimitiveType,
    ReferenceType,
    RuntimeLanguage,
    ScalarType,
    Struct,
    StructType,
    TypeCardinality,
    TypeDeclaration,
    UInt32,
    declare_method,
    declare_property,
    declare_struct,
)

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false, reportIncompatibleMethodOverride=false


@declare_struct(StructType.TYPE, is_final=True)
@final
class Type(Struct):
    """
    A Type in the type system.
    Types compose like a tree with scalars at the leaves:
     - Scalar: a single value (self, Type.scalar_type)
     - List: a dynamic sequence of homogeneous values (Type.value_type)
     - Tuple: a fixed sequence of heterogeneous values (Type.element_types)
     - Array: a fixed n-dimensional sequence of homogeneous values (Type.value_type * Type.dimensions)
     - Map: a dynamic mapping of homogenous keys to homogeneous values (Type.key_type->Type.value_type)
     - Union: a union of heterogeneous values (Type.element_types)
    """

    # cardinality
    cardinality: TypeCardinality = declare_property(
        110,
        is_repr=True,
        description="Cardinality of this Type.",
    )
    key_type: Optional["Type"] = declare_property(
        111,
        is_repr=True,
        description="Key type of this Type (if it's a map).",
    )
    value_type: Optional["Type"] = declare_property(
        112,
        is_repr=True,
        description="Value type of this Type (list, map).",
    )
    element_types: list["Type"] | None = declare_property(
        113,
        is_repr=True,
        description="Element types of this Type (tuple, union).",
    )
    length: Optional[UInt32] = declare_property(
        114,
        is_repr=True,
        description="Length of this Type (string primitive, list, tuple).",
    )
    dimensions: list[UInt32] | None = declare_property(
        115,
        is_repr=True,
        description="Multiple dimensions of this Type (array).",
    )
    # generic_over?
    is_required: bool = declare_property(119, default=True)

    # scalar
    scalar_type: Optional[ScalarType] = declare_property(
        120,
        is_repr=True,
        description="Scalar value type of this Type (primitive, enum, node, struct, etc..).",
    )
    primitive_type: Optional[PrimitiveType] = declare_property(
        121,
        is_repr=True,
        description="Primitive type of this Type (if it's a primitive scalar).",
    )
    enum_type: Optional[EnumType] = declare_property(
        122,
        is_repr=True,
        description="Enum type of this Type (if it's an enum scalar).",
    )
    node_types: list[NodeType] | None = declare_property(
        123,
        is_repr=True,
        description="Node types of this Type (if it's a node reference or node value scalar).",
    )
    struct_type: Optional[StructType] = declare_property(
        124,
        is_repr=True,
        description="Struct type of this Type (if it's a struct scalar).",
    )
    handle_type: Optional[HandleType] = declare_property(
        125,
        is_repr=True,
        description="Handle type of this Type (if it's a handle scalar).",
    )

    @classmethod
    def from_declaration(cls, declaration: "TypeDeclaration") -> "Type":
        """Create a Type from a TypeDeclaration."""
        return cls(
            # cardinality
            cardinality=declaration.cardinality,
            key_type=cls.from_declaration(declaration.key_type) if declaration.key_type else None,
            value_type=cls.from_declaration(declaration.value_type)
            if declaration.value_type
            else None,
            element_types=[cls.from_declaration(t) for t in declaration.element_types]
            if declaration.element_types
            else None,
            # scalar
            scalar_type=declaration.scalar_type,
            primitive_type=declaration.primitive_type,
            enum_type=declaration.enum_type,
            node_types=list(declaration.node_types) if declaration.node_types else None,
            struct_type=declaration.struct_type,
            handle_type=declaration.handle_type,
            # flags
            is_required=declaration.is_required,
        )

    @declare_method(201, is_implemented=True)
    @classmethod
    def of(
        cls, value_or_type: Any, reference_type: ReferenceType | None = ReferenceType.REGULAR
    ) -> "Type":
        """
        Infer the Type of a value or class.
        For values, we try to infer the most specific Type that can represent the value.
        """
        return infer_type(value_or_type, reference_type)


@declare_struct(StructType.STRING_CONSTRAINT)
class StringConstraint(Struct):
    """The constraint of a string."""

    regex: Optional[str] = declare_property(41)
    starts_with: Optional[str] = declare_property(42)
    ends_with: Optional[str] = declare_property(43)


SLUG_REGEX_CHAR = r"a-z0-9-"
SLUG_REGEX = rf"^[{SLUG_REGEX_CHAR}]{{3,}}$"
EMAIL_REGEX = r"^[a-zA-Z0-9_.+\-]+@[a-zA-Z0-9\-]+\.[a-zA-Z0-9\-.]+$"
URL_REGEX = r"^(?:[a-z]+:\/\/)?[\w.-]+\.[a-z]{2,}(?:\/\S*)?$"
PHONE_NUMBER_REGEX = r"^\+?(\d{1,3})?[-.\s]?(\(?\d{1,4}\)?)?[-.\s]?\d{1,4}[-.\s]?\d{1,9}$"


@declare_struct(
    StructType.NUMBER_CONSTRAINT,
    is_final=True,
)
@final
class NumberConstraint(Struct):
    """The constraint of a number."""

    min_value: Optional[Float32] = declare_property(41)
    max_value: Optional[Float32] = declare_property(42)
    step_value: Optional[Float32] = declare_property(43)


@declare_struct(
    StructType.COLLECTION_CONSTRAINT,
    is_final=True,
)
@final
class CollectionConstraint(Struct):
    """The constraint of a collection."""

    min_length: Optional[UInt32] = declare_property(41)
    max_length: Optional[UInt32] = declare_property(42)


TypeConstraint = Union[NumberConstraint, StringConstraint, CollectionConstraint]

_PRIMITIVE_PY_TYPES: tuple[type, ...] = tuple(
    t for t in PRIMITIVE_TYPE_BY_ANNOTATION if isinstance(t, type)
)


@declare_method(301, is_implemented=True)
def infer_type(
    value_or_type: Any, reference_type: ReferenceType | None = ReferenceType.REGULAR
) -> "Type":
    """
    Infer the Type of a value or class.
    For values, we try to infer the most specific Type that can represent the value.
    """
    from ..builtin import Handle, Node, parse_type_declaration
    from .relation import NodeReference

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
        if reference_type == ReferenceType.REGULAR:
            scalar_type = ScalarType.NODE
        elif reference_type == ReferenceType.THIN:
            scalar_type = ScalarType.NODE_ID
        elif reference_type is None:
            scalar_type = ScalarType.NODE
        else:
            assert_never(reference_type)
        return Type(
            cardinality=TypeCardinality.SCALAR,
            scalar_type=scalar_type,
            node_types=[value_or_type.metatype],
        )
    elif isinstance(value_or_type, Struct):
        return Type(
            cardinality=TypeCardinality.SCALAR,
            scalar_type=ScalarType.STRUCT,
            struct_type=value_or_type.metatype,
        )
    elif isinstance(value_or_type, Handle):
        return Type(
            cardinality=TypeCardinality.SCALAR,
            scalar_type=ScalarType.HANDLE,
            handle_type=value_or_type.metatype,
        )
    elif isinstance(value_or_type, Enum):
        return Type(
            cardinality=TypeCardinality.SCALAR,
            scalar_type=ScalarType.ENUM,
            enum_type=ENUM_TYPE_BY_CLASS[type(value_or_type)],
        )
    elif isinstance(value_or_type, OptionDeclaration):
        return Type(
            cardinality=TypeCardinality.SCALAR,
            scalar_type=ScalarType.ENUM,
            enum_type=ENUM_TYPE_BY_CLASS[value_or_type.component],
        )
    elif isinstance(value_or_type, _PRIMITIVE_PY_TYPES):
        primitive_type = PRIMITIVE_TYPE_BY_ANNOTATION.get(type(value_or_type))
        if primitive_type is None:
            raise ValueError(f"no primitive type for {type(value_or_type)!r}")
        return Type(
            cardinality=TypeCardinality.SCALAR,
            scalar_type=ScalarType.PRIMITIVE,
            primitive_type=primitive_type,
        )

    # collections
    if isinstance(value_or_type, tuple):
        # tuples are heterogeneous
        element_types = [infer_type(elem, reference_type=reference_type) for elem in value_or_type]
        return Type(
            cardinality=TypeCardinality.TUPLE,
            element_types=element_types,  # type: ignore
        )
    elif isinstance(value_or_type, list):
        assert value_or_type, f"cannot infer type of empty list: {value_or_type!r}"
        # lists are homogeneous - infer from first element
        element_type = infer_type(value_or_type[0], reference_type=reference_type)
        return Type(
            cardinality=TypeCardinality.LIST,
            value_type=element_type,
        )
    elif isinstance(value_or_type, dict):
        assert value_or_type, f"cannot infer type of empty dict: {value_or_type!r}"
        sample_key = next(iter(value_or_type))
        sample_value = value_or_type[sample_key]
        key_type = infer_type(sample_key, reference_type=reference_type)
        value_type = infer_type(sample_value, reference_type=reference_type)
        return Type(
            cardinality=TypeCardinality.MAP,
            key_type=key_type,
            value_type=value_type,
        )

    #
    # Annotations
    #

    # parse as annotation
    type_decl = parse_type_declaration(value_or_type, is_builtin=False)
    return type_decl.to_type()


@declare_method(
    302,
    is_implemented=True,
    languages=(RuntimeLanguage.PYTHON,),
)
def invert_type(type: "Type") -> "Any":
    """
    Get the type annotation that corresponds to a Type.
    """
    # scalar
    if type.cardinality == TypeCardinality.SCALAR:
        return invert_type_scalar(type)
    # list
    elif type.cardinality == TypeCardinality.LIST:
        assert type.value_type is not None, f"no value type for: {type!r}"
        return list[invert_type(type.value_type)]  # type: ignore
    # tuple
    elif type.cardinality == TypeCardinality.TUPLE:
        assert type.element_types is not None, f"no element types for: {type!r}"
        element_annotations = [invert_type(t) for t in type.element_types]
        return tuple[*element_annotations]  # type: ignore
    # map
    elif type.cardinality == TypeCardinality.MAP:
        assert type.key_type is not None, f"no key type for: {type!r}"
        assert type.value_type is not None, f"no value type for: {type!r}"
        key_annotation = invert_type(type.key_type)
        value_annotation = invert_type(type.value_type)
        return dict[key_annotation, value_annotation]
    #
    else:
        assert_never(type.cardinality)


def invert_type_scalar(type: "Type") -> "Any":
    """
    Get the Python type that corresponds to a scalar Type.
    """
    assert type.cardinality == TypeCardinality.SCALAR, f"not a scalar type: {type!r}"
    assert type.scalar_type is not None, f"no scalar type for: {type!r}"

    # primitive
    if type.scalar_type == ScalarType.PRIMITIVE:
        assert type.primitive_type is not None, f"no primitive type for: {type!r}"
        return PRIMITIVE_PY_ANNOTATION_BY_TYPE[type.primitive_type]
    # enum
    elif type.scalar_type == ScalarType.ENUM:
        assert type.enum_type is not None, f"no enum type for: {type!r}"
        return ENUM_CLASS_BY_TYPE[type.enum_type]
    # node
    elif type.scalar_type == ScalarType.NODE:
        assert type.node_types is not None, f"no node types for: {type!r}"
        return NODE_CLASS_BY_TYPE[type.node_types[0]]
    # node reference
    elif type.scalar_type in (ScalarType.NODE_REFERENCE, ScalarType.NODE_ID):
        assert type.node_types is not None, f"no node types for: {type!r}"
        if len(type.node_types) == 1:
            return NODE_CLASS_BY_TYPE[type.node_types[0]]
        else:
            return Union[*tuple(NODE_CLASS_BY_TYPE[t] for t in type.node_types)]  # type: ignore
    # struct
    elif type.scalar_type == ScalarType.STRUCT:
        assert type.struct_type is not None, f"no struct type for: {type!r}"
        return STRUCT_CLASS_BY_TYPE[type.struct_type]
    # handle
    elif type.scalar_type == ScalarType.HANDLE:
        assert type.handle_type is not None, f"no handle type for: {type!r}"
        return HANDLE_CLASS_BY_TYPE[type.handle_type]
    # union
    elif type.scalar_type == ScalarType.UNION:
        assert type.element_types is not None, f"no element types for: {type!r}"
        element_annotations = [invert_type_scalar(t) for t in type.element_types]
        return Union[*element_annotations]  # type: ignore
    else:
        assert_never(type.scalar_type)
