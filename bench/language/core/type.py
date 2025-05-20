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

from bench.language.registry import BENCH_CLASS_BY_TYPE, BENCH_TYPE_BY_CLASS

from .const import (
    PRIMITIVE_TYPE_BY_PY_TYPE,
    PY_TYPE_BY_PRIMITIVE_TYPE,
    BenchType,
    BuiltinEnum,
    EnumType,
    NodeType,
    PrimitiveType,
    PrimitiveValue,
    StructType,
    enum_,
    is_enum_type,
    is_node_type,
    is_struct_type,
)
from .node import Node, NodeReference
from .object import BuiltinObject, get_tk_b64_from_ck, object_
from .property import p_internal, p_regular
from .struct import Struct, struct_
from .validation import TypeConstraintIn

if TYPE_CHECKING:
    from bench.language import Field, FileType, Value

# pyright: reportIncompatibleVariableOverride=false, reportIncompatibleMethodOverride=false

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


@enum_(EnumType.TYPE_TYPE)
class TypeType(BuiltinEnum):
    """The 'kind' of a Type."""

    PRIMITIVE = 1
    STRUCT = 2
    NODE = 3
    ENUM = 4


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


@struct_(StructType.TYPE_CONSTRAINT)
class TypeConstraint(Struct):
    """
    A constraint on the values of a type. :TypeConstraint
    Basically, a TypeConstraint is a more restricted form of a condition Expression.
    """

    # numeric
    min_value: Optional[float] = p_regular(40)
    max_value: Optional[float] = p_regular(41)
    step_value: Optional[float] = p_regular(42)
    # sequence-ish
    min_length: Optional[int] = p_regular(50)
    max_length: Optional[int] = p_regular(51)
    # string-ish
    regex: Optional[str] = p_regular(60)
    starts_with: Optional[str] = p_regular(61)
    ends_with: Optional[str] = p_regular(62)
    # node-ish
    node_types: list["NodeType"] = p_regular(71)
    node_max_depth: Optional[int] = p_regular(73)
    # e.g., for Text (number of lines), ...


constraint = TypeConstraintIn


# nocheckin: revamp Type, TypeFormat, TypeConstraint cascading Type bases, Maps, ...
#  TypeConstraint.node_subtypes feels wrong, need Node-specific constraints?
#  specific node/struct/custom constraints?
@object_()
class TypeBase(BuiltinObject):
    """
    A Type describes the shape of a value.
    """

    # type identity
    kind: TypeType = p_internal(40)
    primitive_type: Optional[PrimitiveType] = p_regular(41)
    node_type: Optional[NodeType] = p_regular(42)
    struct_type: Optional[StructType] = p_regular(43)
    enum_type: Optional[EnumType] = p_regular(44)
    base_type: Optional["Node"] = p_regular(45)
    if TYPE_CHECKING:
        base_type_id: Optional[UUID] = None
        base_type_ptr: Optional["NodeReference"] = None

    # metadata
    default: Optional["Value"] = p_regular(50)
    constraint: Optional["TypeConstraint"] = p_regular(55)

    # flags
    is_required: bool = p_regular(60, default=False)
    is_list: bool = p_regular(61, default=False)
    is_secret: bool = p_regular(62, default=False)

    def morph_to(
        self,
        typ: "TypeIn",
        constraint: TypeConstraintIn | TypeConstraint | None = None,
        is_required: bool = False,
        is_list: bool = False,
    ):
        """Change this type to another type."""
        typ = to_type(typ, constraint=constraint, is_required=is_required, is_list=is_list)
        for prop in TypeBase.__declared_properties__.values():
            new_typ_value = getattr(typ, prop.name)
            old_typ_value = getattr(self, prop.name)
            if new_typ_value != old_typ_value:
                setattr(self, prop.name, new_typ_value)

    @property
    def supports_list(self) -> bool:  # :ListableTypes
        """Whether this type supports lists."""
        if self.kind in (  # noqa: SIM114
            TypeType.NODE,
            TypeType.ENUM,
        ):
            return True
        elif self.primitive_type in (  # noqa: SIM103
            PrimitiveType.FLOAT32,
            PrimitiveType.FLOAT64,
            PrimitiveType.INT32,
            PrimitiveType.INT64,
            PrimitiveType.STRING,
            PrimitiveType.UUID,
            PrimitiveType.DATETIME,
            PrimitiveType.DATE,
            PrimitiveType.TIME,
            PrimitiveType.DURATION,
        ):
            return True
        else:
            return False

    @property
    def is_scalar(self) -> bool:
        """Whether this type is a scalar (not a list)."""
        return not self.is_list

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


@struct_(StructType.TYPE)
class Type(Struct, TypeBase):
    """A Type in the type system."""

    # redirect so we get TypeBase.__content_str__ (not Struct.__content_str__)
    __content_str__ = TypeBase.__content_str__  # type: ignore

    @staticmethod
    def from_type(
        typ: "TypeIn", constraint: "TypeConstraintIn | TypeConstraint | None" = None
    ) -> "Type":
        """Converts a type-like object to a TypeInfo."""
        typ = to_type_scalar(typ)
        if constraint is not None:
            if isinstance(constraint, TypeConstraintIn):
                constraint = constraint.into()
            typ.constraint = constraint
        return typ


#
# Convenience to type utilities
#

TypeIn = Union[
    "TypeBase",
    "Node",
    "BuiltinEnum",
    "PrimitiveType",
    "BenchType",
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
        Choice,
        Flow,
        FlowEdge,
        Schema,
        Table,
    )

    if isinstance(type_in, Block) and (node := type_in.node) is not None:
        type_in = cast(TypeIn, node)  # unpack inner node automatically

    if isinstance(type_in, TypeBase):
        return cast("Type", type_in)
    elif isinstance(type_in, (Schema, Choice, Flow, Action, FlowEdge, Table, Agent)):
        raise NotImplementedError
        type_scalar = type_in.to_type_maybe()
        if type_scalar is not None:
            assert isinstance(type_scalar, Type), f"expected Type, got {type_scalar!r}"
            return type_scalar
    elif isinstance(type_in, PrimitiveType):
        return Type(kind=TypeType.PRIMITIVE, primitive_type=type_in)
    elif isinstance(type_in, (NodeType, StructType, EnumType, BenchType)):
        if is_node_type(type_in):
            return Type(kind=TypeType.NODE, node_type=type_in)
        elif is_struct_type(type_in):
            return Type(kind=TypeType.STRUCT, struct_type=type_in)
        elif is_enum_type(type_in):
            return Type(kind=TypeType.ENUM, enum_type=type_in)
    elif isinstance(type_in, type):
        primitive_type = PRIMITIVE_TYPE_BY_PY_TYPE.get(type_in)
        if primitive_type:
            return Type(kind=TypeType.PRIMITIVE, primitive_type=primitive_type)
        bench_type = BENCH_TYPE_BY_CLASS.get(cast(Any, type_in))
        if bench_type is not None:
            if is_node_type(bench_type):
                return Type(kind=TypeType.NODE, node_type=bench_type)
            elif is_struct_type(bench_type):
                return Type(kind=TypeType.STRUCT, struct_type=bench_type)
            elif is_enum_type(bench_type):
                return Type(kind=TypeType.ENUM, enum_type=bench_type)
        elif type_in == Node:
            return Type(kind=TypeType.NODE)

    raise ValueError(f"unsupported type {type_in!r}")


def to_type(
    type_in: TypeIn,
    *,
    constraint: TypeConstraintIn | TypeConstraint | None = None,
    is_required: bool = False,
    is_list: bool = False,
) -> Type:
    """Converts a TypeIn into a Type."""
    type_scalar = to_type_scalar(type_in)
    if isinstance(constraint, TypeConstraintIn):
        constraint = constraint.into()
    type_scalar.constraint = constraint
    type_scalar.is_required = is_required
    type_scalar.is_list = is_list
    return type_scalar


def reverse_type_scalar(typ: TypeBase) -> TypeIn | None:
    """
    Reverses a Type into a TypeIn as closely as possible.
    Does not consider non-scalar properties (is_list, is_required, etc.)
    """
    if typ.kind == TypeType.PRIMITIVE:
        assert typ.primitive_type is not None, f"missing primitive type for {typ!r}"
        primitive_cls = PY_TYPE_BY_PRIMITIVE_TYPE.get(typ.primitive_type)
        if primitive_cls and PRIMITIVE_TYPE_BY_PY_TYPE.get(primitive_cls) == typ.primitive_type:
            return primitive_cls
        else:
            return typ.primitive_type
    elif typ.kind in (TypeType.NODE, TypeType.STRUCT, TypeType.ENUM):
        if typ.kind == TypeType.NODE and typ.node_type is None:
            return Node
        assert typ.node_type is not None, f"missing node type for {typ!r}"
        node_cls = BENCH_CLASS_BY_TYPE[typ.node_type]
        return cast(TypeIn, node_cls)

    return None  # couldn't reverse
