import typing
from typing import (
    TYPE_CHECKING,
    Any,
    NamedTuple,
    Optional,
    Sequence,
    Union,
    cast,
)
from uuid import UUID

import structlog
from opentelemetry import trace

from bench.language.registry import BENCH_CLASS_BY_TYPE, BENCH_TYPE_BY_CLASS
from bench.utils.func import decode_b64vlq, encode_b64vlq

from .const import (
    CK_LENGTH_B64,
    PRIMITIVE_TYPE_BY_PY_TYPE,
    PY_TYPE_BY_PRIMITIVE_TYPE,
    BenchType,
    BuiltinEnum,
    EnumType,
    FieldType,
    NodeMode,
    NodeType,
    PrimitiveType,
    PrimitiveValue,
    StructType,
    TypeFormat,
    TypeKind,
    is_enum_type,
    is_node_type,
    is_struct_type,
)
from .node import Node, NodeReference, TypeBaseNode
from .object import BuiltinObject, get_tk_b64_from_ck, object_
from .property import Property, p_internal, p_regular, p_runtime, p_value_packed, p_value_runtime
from .struct import Struct, struct_
from .trait import TYPE_BASE_NODE_TYPES, FieldBaseNode
from .validation import TypeConstraintIn
from .value import SomeValue

if typing.TYPE_CHECKING:
    from bench.language import (
        Action,
        Agent,
        Block,
        Choice,
        Class,
        Database,
        Expression,
        Field,
        FileType,
        Flow,
        Transition,
    )

# pyright: reportIncompatibleVariableOverride=false, reportIncompatibleMethodOverride=false

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


LETTER_BY_TYPE_KIND: dict[TypeKind, str] = {
    TypeKind.PRIMITIVE: "p",
    TypeKind.STRUCT: "s",
    TypeKind.NODE: "n",
    TypeKind.BASED_NODE: "n",  # overlap with TypeKind.NODE
    TypeKind.ENUM: "e",
    TypeKind.CUSTOM_OBJECT: "o",
    TypeKind.PARTIAL_OBJECT: "r",
}
TYPE_KIND_BY_LETTER: dict[str, TypeKind] = {
    "p": TypeKind.PRIMITIVE,
    "s": TypeKind.STRUCT,
    "n": TypeKind.NODE,
    "e": TypeKind.ENUM,
    "o": TypeKind.CUSTOM_OBJECT,
    "r": TypeKind.PARTIAL_OBJECT,
}


class TypeIdentity(NamedTuple):
    kind: TypeKind
    primitive_type: Optional[PrimitiveType] = None
    bench_type: Optional[BenchType] = None
    base_type_ptr: Optional[NodeReference] = None
    base_field_types: list["FieldType"] | None = None
    property_field_types: list["FieldType"] | None = None
    is_required: bool = False
    is_list: bool = False
    is_secret: bool = False
    format: Optional[TypeFormat] = None
    constraint: Optional["TypeConstraint"] = None


def encode_type_identity(typ: "IsType | TypeIdentity") -> str:
    """
    Encodes the type identity into a key for storage & implicit typing.
    Format is <kind>[id] (with id encoded as base64).
    :TypeInfoEncoding
    """

    value: str
    if typ.kind == TypeKind.PRIMITIVE:
        assert typ.primitive_type is not None, f"missing primitive type for {typ!r}"
        value = encode_b64vlq(typ.primitive_type.id)
    elif typ.kind == TypeKind.NODE or typ.kind == TypeKind.BASED_NODE:
        value = ""  # joint type identity for nodes
    elif typ.kind == TypeKind.STRUCT or typ.kind == TypeKind.ENUM:
        assert typ.bench_type is not None, f"missing bench type for {typ!r}"
        value = encode_b64vlq(typ.bench_type.id)
    elif typ.kind == TypeKind.CUSTOM_OBJECT:
        assert typ.base_type_ptr is not None, f"missing base type for {typ!r}"
        assert typ.base_type_ptr.ck is not None, f"missing ck for {typ.base_type_ptr!r}"
        value = encode_b64vlq(typ.base_type_ptr.ck.int)
    elif typ.kind == TypeKind.PARTIAL_OBJECT:
        value = encode_b64vlq(typ.bench_type.id) if typ.bench_type else ""
    else:
        raise ValueError(f"unsupported type kind {typ.kind.bench_name} for {typ!r}")

    prefix: str
    prefix = LETTER_BY_TYPE_KIND[typ.kind].upper() if typ.is_list else LETTER_BY_TYPE_KIND[typ.kind]
    if typ.is_secret:
        prefix = "!" + prefix
    return f"{prefix}{value}"


def decode_type_identity(key: str) -> "TypeIdentity":
    """Decodes the type-related info back from the identity key. See encode. :TypeInfoEncoding"""
    # prefix
    if key[0] == "!":
        key = key[1:]
        is_secret = True
    else:
        is_secret = False
    if key[0].isupper():
        is_list = True
        kind = TYPE_KIND_BY_LETTER.get(key[0].lower())
    else:
        is_list = False
        kind = TYPE_KIND_BY_LETTER.get(key[0])
    value = key[1:]

    # value
    if kind == TypeKind.PRIMITIVE.value:
        primitive_type = PrimitiveType(decode_b64vlq(value))
        return TypeIdentity(
            kind=TypeKind.PRIMITIVE,
            primitive_type=primitive_type,
            is_list=is_list,
            is_secret=is_secret,
        )
    elif kind == TypeKind.STRUCT.value or kind == TypeKind.ENUM.value:
        bench_type = BenchType(decode_b64vlq(value))  # type: ignore
        return TypeIdentity(
            kind=TypeKind(kind), bench_type=bench_type, is_list=is_list, is_secret=is_secret
        )
    elif kind == TypeKind.NODE or kind == TypeKind.BASED_NODE.value:
        return TypeIdentity(kind=TypeKind(kind), is_list=is_list, is_secret=is_secret)
    elif kind == TypeKind.CUSTOM_OBJECT.value:
        base_id = UUID(int=decode_b64vlq(value))
        base_type_ptr = NodeReference(
            node_type=NodeType.BLOCK, id=base_id, ck=base_id, _skip_validate_self=True
        )
        return TypeIdentity(
            kind=TypeKind.CUSTOM_OBJECT,
            base_type_ptr=base_type_ptr,
            is_list=is_list,
            is_secret=is_secret,
        )
    elif kind == TypeKind.PARTIAL_OBJECT.value:
        bench_type = BenchType(decode_b64vlq(value)) if value else None  # type: ignore
        return TypeIdentity(
            kind=TypeKind.PARTIAL_OBJECT,
            bench_type=bench_type,
            is_list=is_list,
            is_secret=is_secret,
        )

    raise ValueError(f"unsupported type kind {kind} for {key!r}")


LETTER_BY_FIELD_TYPE: dict[FieldType, str] = {
    FieldType.MEMBER: "M",
    FieldType.INPUT: "I",
    FieldType.OUTPUT: "O",
}
FIELD_TYPE_BY_LETTER: dict[str, FieldType] = {
    "M": FieldType.MEMBER,
    "I": FieldType.INPUT,
    "O": FieldType.OUTPUT,
}

STORAGE_KEY_PREFIX_LENGTH = CK_LENGTH_B64 + 1


def encode_storage_key(field: "Field") -> str:
    """Gets the key used to identify values of this field in storage. :FieldStorageKey"""
    return f"{LETTER_BY_FIELD_TYPE[field.type]}{get_tk_b64_from_ck(field.ck)}{field.identity_key}"


def get_field_type(storage_key: str) -> FieldType:
    return FIELD_TYPE_BY_LETTER[storage_key[0]]


@struct_(StructType.TYPE_CONSTRAINT)
class TypeConstraint(Struct):
    """
    A constraint on the values of a type. :TypeConstraint
    Basically, a TypeConstraint is a more restricted form of a condition Expression.
    """

    # numeric
    min_value: Optional[float] = p_regular(40, require=False, default=None)
    max_value: Optional[float] = p_regular(41, require=False, default=None)
    step_value: Optional[float] = p_regular(42, require=False, default=None)
    # sequence-ish
    min_length: Optional[int] = p_regular(50, require=False, default=None)
    max_length: Optional[int] = p_regular(51, require=False, default=None)
    # string-ish
    regex: Optional[str] = p_regular(60, require=False, default=None)
    starts_with: Optional[str] = p_regular(61, require=False, default=None)
    ends_with: Optional[str] = p_regular(62, require=False, default=None)
    # node-ish
    node_mode: Optional[NodeMode] = p_regular(70, require=False, default=None)
    node_types: list["NodeType"] = p_regular(71, array=True)
    node_scope: list["Node"] = p_regular(72, require=False, array=True, references="any")
    node_max_depth: Optional[int] = p_regular(73, require=False, default=None)
    # NOTE :Architecture: TypeConstraint.node_subtypes feels wrong, need Node-specific constraints?
    node_subtypes: list[int] = p_regular(74, array=True)
    # specific node-ish
    # ...?


constraint = TypeConstraintIn


@object_()
class IsType(BuiltinObject):
    """
    A Type describes the properties and shape of a value.

    A Type is of one of:
       1. Primitive (= column type, value is scalar, like int32, string, bool, datetime)
       2. Struct (Struct like Expression, File, Path, Text, Code)
       3. Node (NodeReference, like Package, Block, Field, Record, Run, Signal)
       4. Enum (builtin BuiltinEnum, like FieldKind, NodeType, BenchType, EnumType)
       5. Based Node (NodeReference,  an 'instance' of the block)
       6. Custom Object (value is CustomObject, like Action outputs, Record value)
       7. Partial Object (value is a CustomObject + partial Node, like Record partials, CreateActions)
       8. Literal (only allowable value is the type itself / or some constant value)
       9. Union (type is union of Field children with oneof=self)

    Types may also specify:
       - field type, narrowing the Fields included from the base type (if any)
       - condition which instances must satisfy
       - constraints (simple conditions the value must satisfy)
       - combination flags for arrays, optionals
    """

    # type identity
    kind: TypeKind = p_internal(40)
    primitive_type: Optional[PrimitiveType] = p_regular(41, default=None)
    bench_type: Optional[BenchType] = p_regular(42, default=None)
    base_type: Union[TypeBaseNode, None] = p_regular(
        43, array=False, require=False, default=None, references=TYPE_BASE_NODE_TYPES.tuple
    )
    if TYPE_CHECKING:
        base_type_id: Optional[UUID] = None
        base_type_ptr: Optional["NodeReference"] = None
    base_field_types: list["FieldType"] = p_internal(44, require=False, array=True)
    property_field_types: list["FieldType"] = p_internal(45, require=False, array=True)

    # metadata
    default_packed: Optional[Any] = p_value_packed(50)
    default = p_value_runtime(packed=50, typ=lambda self: cast("IsType", self))
    format: Optional["TypeFormat"] = p_regular(53, default=None)
    condition: Optional["Expression"] = p_regular(
        54, require=False, array=False, default=None, struct=StructType.EXPRESSION
    )
    constraint: Optional["TypeConstraint"] = p_regular(
        55, require=False, array=False, default=None, struct=StructType.TYPE_CONSTRAINT
    )

    # flags
    is_required: bool = p_regular(60, default=False)
    is_list: bool = p_regular(61, default=False)
    is_secret: bool = p_regular(62, default=False)
    # is_streaming?

    _from_property: Optional["Property"] = p_runtime(default=None)

    def __content_str__(self) -> str:
        base_type = self.base_type
        if base_type is not None:
            if self.bench_type is not None:
                info_str = f"{self.bench_type.bench_name}->{base_type.absolute_path}"
            else:
                info_str = f"{self.kind.bench_name}->{base_type.absolute_path}"
        elif self.bench_type is not None:
            info_str = self.bench_type.bench_name
        elif self.primitive_type is not None:
            info_str = self.primitive_type.bench_name
        else:
            info_str = self.kind.bench_name

        clauses = []
        if self.kind == TypeKind.PARTIAL_OBJECT:
            if not info_str.startswith("Partial"):
                info_str = f"Partial{info_str}"
            if self.property_field_types:
                clauses.append(
                    f"Property={'|'.join(f.bench_name for f in self.property_field_types)}"
                )
        if self.condition is not None:
            clauses.append(repr(self.condition))
        if self.is_list:
            clauses.append("is_list")
        if self.is_required:
            clauses.append("is_required")
        if self.is_secret:
            clauses.append("is_secret")
        if self.base_field_types:
            clauses.append(f"Field={'|'.join(f.bench_name for f in self.base_field_types)}")
        if self.constraint is not None:
            constraint_str = self.constraint.__content_str__()
            if constraint_str:
                clauses.append(constraint_str)

        if clauses:
            info_str += f" [{', '.join(clauses)}]"
        if self._from_property:
            info_str += f" from {self._from_property!s}"

        return info_str

    def __call__(self, *args, **kwargs) -> "SomeValue":
        """Converts the given value to this type."""
        # TODO :Cleanup :Architecture: TypeInfo.__call__ feels a lot like coerce_value
        #  But it's not quite the same. Here we want to error if we can't coerce, return full nodes, etc.
        if self.kind == TypeKind.PRIMITIVE:
            py_type = PY_TYPE_BY_PRIMITIVE_TYPE.get(cast(PrimitiveType, self.primitive_type))
            assert py_type is not None, f"{self!r} does not have a python type"
            return py_type(*args, **kwargs)
        elif self.kind == TypeKind.BASED_NODE:
            if self.bench_type == NodeType.FIELD:
                assert self.base_type is not None, f"missing base type for {self!r}"
                field = cast(FieldBaseNode, self.base_type).fields.get(*args, **kwargs)
                if field is None:
                    raise ValueError(f"no field {args!r} in {self.base_type!r}")
                return field
        elif self.kind == TypeKind.CUSTOM_OBJECT or self.kind == TypeKind.PARTIAL_OBJECT:
            from .value import coerce_custom_object_scalar

            return coerce_custom_object_scalar(kwargs, self, as_packed=True)

        raise ValueError(f"cannot create {self!r} (resolved={self!r}) directly")

    def morph_to(
        self,
        typ: "TypeIn",
        constraint: TypeConstraintIn | TypeConstraint | None = None,
        is_required: bool = False,
        is_list: bool = False,
    ):
        """Change this type to another type."""
        typ = to_type(typ, constraint=constraint, is_required=is_required, is_list=is_list)
        for prop in IsType.__declared_properties__.values():
            new_typ_value = getattr(typ, prop.name)
            old_typ_value = getattr(self, prop.name)
            if new_typ_value != old_typ_value:
                setattr(self, prop.name, new_typ_value)

    @property
    def supports_list(self) -> bool:  # :ListableTypes
        """Whether this type supports lists."""
        if self.kind in (  # noqa: SIM114
            TypeKind.NODE,
            TypeKind.BASED_NODE,
            TypeKind.ENUM,
            TypeKind.CUSTOM_OBJECT,
            TypeKind.PARTIAL_OBJECT,
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
        if (base_type := self.base_type) is not None and base_type.metatype != NodeType.CHOICE:
            return cast(FieldBaseNode, base_type).fields
        else:
            return ()

    @property
    def _fields(self) -> Sequence["Field"]:
        if not self.base_field_types:
            return self._base_fields
        else:
            return tuple(f for f in self._base_fields if f.type in self.base_field_types)

    def _get_field(self, ident: str) -> Optional["Field"]:
        """Resolves a field in this type by an identifier (name or py_name)"""
        for field in self._base_fields:
            if (field.code_name == ident or field.name == ident) and (
                not self.base_field_types or field.type in self.base_field_types
            ):
                return field
        return None

    def _get_field_by_key(self, key: str) -> Optional["Field"]:
        """Resolves a field in this type by its tk"""
        for field in self._base_fields:
            if field.key == key:
                return field
        return None


@struct_(StructType.TYPE)
class Type(Struct, IsType):
    """A Type in the type system."""

    # redirect so we get TypeBase.__content_str__ (not Struct.__content_str__)
    __content_str__ = IsType.__content_str__  # type: ignore

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
    "IsType",
    "Block",
    "Database",
    "Class",
    "Choice",
    "Flow",
    "Agent",
    "BuiltinEnum",
    "Transition",
    "Action",
    "PrimitiveType",
    "BenchType",
    "TypeFormat",
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
        Class,
        Database,
        FileType,
        Flow,
        Transition,
    )

    if isinstance(type_in, Block) and (node := type_in.node) is not None:
        type_in = cast(TypeIn, node)  # unpack inner node automatically

    if isinstance(type_in, IsType):
        return cast("Type", type_in)
    elif isinstance(type_in, (Class, Choice, Flow, Action, Transition, Database, Agent)):
        type_scalar = type_in.to_type_maybe()
        if type_scalar is not None:
            assert isinstance(type_scalar, Type), f"expected Type, got {type_scalar!r}"
            return type_scalar
    elif isinstance(type_in, PrimitiveType):
        return Type(kind=TypeKind.PRIMITIVE, primitive_type=type_in)
    elif isinstance(type_in, (NodeType, StructType, EnumType, BenchType)):
        if is_node_type(type_in):
            return Type(kind=TypeKind.NODE, bench_type=type_in)
        elif is_struct_type(type_in):
            return Type(kind=TypeKind.STRUCT, bench_type=type_in)
        elif is_enum_type(type_in):
            return Type(kind=TypeKind.ENUM, bench_type=type_in)
    elif isinstance(type_in, TypeFormat):
        return Type(kind=TypeKind.PRIMITIVE, primitive_type=type_in.primitive_type, format=type_in)
    elif isinstance(type_in, FileType):
        return Type(
            kind=TypeKind.NODE,
            bench_type=NodeType.FILE,
            constraint=TypeConstraint(node_subtypes=[type_in]),
        )
    elif isinstance(type_in, type):
        primitive_type = PRIMITIVE_TYPE_BY_PY_TYPE.get(type_in)
        if primitive_type:
            return Type(kind=TypeKind.PRIMITIVE, primitive_type=primitive_type)
        bench_type = BENCH_TYPE_BY_CLASS.get(cast(Any, type_in))
        if bench_type is not None:
            if is_node_type(bench_type):
                return Type(kind=TypeKind.NODE, bench_type=bench_type)
            elif is_struct_type(bench_type):
                return Type(kind=TypeKind.STRUCT, bench_type=bench_type)
            elif is_enum_type(bench_type):
                return Type(kind=TypeKind.ENUM, bench_type=bench_type)
        elif type_in == Node:
            return Type(kind=TypeKind.NODE)

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


def reverse_type_scalar(typ: IsType) -> TypeIn | None:
    """
    Reverses a Type into a TypeIn as closely as possible.
    Does not consider non-scalar properties (is_list, is_required, etc.)
    """
    if typ.kind == TypeKind.PRIMITIVE:
        assert typ.primitive_type is not None, f"missing primitive type for {typ!r}"
        if typ.format is not None:
            return typ.format
        primitive_cls = PY_TYPE_BY_PRIMITIVE_TYPE.get(typ.primitive_type)
        if primitive_cls and PRIMITIVE_TYPE_BY_PY_TYPE.get(primitive_cls) == typ.primitive_type:
            return primitive_cls
        else:
            return typ.primitive_type
    elif typ.kind in (TypeKind.NODE, TypeKind.STRUCT, TypeKind.ENUM):
        if typ.constraint is not None:
            if len(typ.constraint.node_subtypes) == 1:
                return cast(TypeIn, typ.constraint.node_subtypes[0])
        if typ.kind == TypeKind.NODE and typ.bench_type is None:
            return Node
        assert typ.bench_type is not None, f"missing bench type for {typ!r}"
        bench_cls = BENCH_CLASS_BY_TYPE[typ.bench_type]
        return cast(TypeIn, bench_cls)
    elif typ.kind == TypeKind.BASED_NODE:
        assert typ.base_type is not None, f"missing base type for {typ!r}"
        return typ.base_type

    return None  # couldn't reverse
