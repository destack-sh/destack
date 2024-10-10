import typing
from typing import (
    TYPE_CHECKING,
    Any,
    Collection,
    Optional,
    Sequence,
    Type,
    Union,
    cast,
    final,
    override,
)
from uuid import UUID

import structlog
from opentelemetry import trace

from bench.language.const import (
    PRIMITIVE_TYPE_BY_PY_TYPE,
    PY_TYPE_BY_PRIMITIVE_TYPE,
    TK_LENGTH_B64,
    BenchType,
    BlockType,
    EnumType,
    FieldZone,
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
from bench.language.expression import _TypeQueryBuilder
from bench.language.graph import NodeList
from bench.language.node import (
    BuiltinObject,
    HasNodeBase,
    Node,
    NodeReference,
    SourceNode,
    Struct,
    get_tk_b64_from_ck,
    get_tk_b64_from_ptr,
    local_node_,
    object_,
    pad_ck_from_tk_b64,
    struct_,
)
from bench.language.property import (
    Property,
    p_internal,
    p_node_parent,
    p_regular,
    p_runtime,
    p_value_packed,
    p_value_runtime,
)
from bench.language.setup import BENCH_CLASS_BY_TYPE, BENCH_TYPE_BY_CLASS
from bench.language.validation import (
    NAME_CONSTRAINT,
    TypeConstraintIn,
    ValidationHandler,
)
from bench.language.value import SomeValue, coerce_value_object_scalar
from bench.proto.wire import AnyNodeData, FieldData, NodeReferenceData
from bench.utils.fractional import INTEGER_ZERO
from bench.utils.func import decode_b64vlq, encode_b64vlq

if typing.TYPE_CHECKING:
    from bench.language import (
        Block,
        Expression,
        FileFormat,
        FileType,
        Icon,
        Step,
        StepType,
        Text,
        ViewType,
    )

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


LETTER_BY_TYPE_KIND: dict[TypeKind, str] = {
    TypeKind.PRIMITIVE: "p",
    TypeKind.STRUCT: "s",
    TypeKind.NODE: "n",
    TypeKind.BASED_NODE: "n",  # overlap with TypeKind.NODE
    TypeKind.ENUM: "e",
    TypeKind.OBJECT: "o",
}
TYPE_KIND_BY_LETTER: dict[str, TypeKind] = {
    "p": TypeKind.PRIMITIVE,
    "s": TypeKind.STRUCT,
    "n": TypeKind.NODE,
    "e": TypeKind.ENUM,
    "o": TypeKind.OBJECT,
}


def get_implied_type_kind(typ: "TypeInfoBase") -> TypeKind | None:
    """Figure out which 'kind' of type is implied by the type info (ignoring its current 'kind')"""
    if typ.primitive_type:
        return TypeKind.PRIMITIVE
    elif typ.bench_type:
        if is_node_type(typ.bench_type):
            if typ.base_type_ptr:
                return TypeKind.BASED_NODE
            else:
                return TypeKind.NODE
        elif is_struct_type(typ.bench_type):
            return TypeKind.STRUCT
        elif is_enum_type(typ.bench_type):
            return TypeKind.ENUM
    elif typ.base_type_ptr:
        return TypeKind.ALIAS
    # couldn't figure it out
    return None


def encode_type_identity(typ: "TypeInfoBase") -> str | None:
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
    elif typ.kind == TypeKind.OBJECT:
        assert typ.base_type_ptr is not None, f"missing base type for {typ!r}"
        value = get_tk_b64_from_ptr(typ.base_type_ptr)
    else:
        return None

    prefix: str
    prefix = LETTER_BY_TYPE_KIND[typ.kind].upper() if typ.is_list else LETTER_BY_TYPE_KIND[typ.kind]
    if typ.is_secret:
        prefix = "!" + prefix
    return f"{prefix}{value}"


def decode_type_identity(key: str) -> "TypeInfoBase":
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
        return TypeInfo(
            kind=TypeKind.PRIMITIVE,
            primitive_type=primitive_type,
            is_list=is_list,
            is_secret=is_secret,
        )
    elif kind == TypeKind.STRUCT.value or kind == TypeKind.ENUM.value:
        bench_type = BenchType(decode_b64vlq(value))  # type: ignore
        return TypeInfo(
            kind=TypeKind(kind), bench_type=bench_type, is_list=is_list, is_secret=is_secret
        )
    elif kind == TypeKind.NODE or kind == TypeKind.BASED_NODE.value:
        return TypeInfo(kind=TypeKind(kind), is_list=is_list, is_secret=is_secret)
    elif kind == TypeKind.OBJECT.value:
        base_type_ptr = NodeReference(
            node_type=NodeType.BLOCK, ck=pad_ck_from_tk_b64(value[:TK_LENGTH_B64])
        )
        return TypeInfo(
            kind=TypeKind.OBJECT, base_type_ptr=base_type_ptr, is_list=is_list, is_secret=is_secret
        )

    raise ValueError(f"unsupported type kind {kind}")


def encode_storage_key(field: "Field") -> str:
    """Gets the key used to identify values of this field in storage. :FieldStorageKey"""
    resolved_field = field._to_resolved()
    return f"{get_tk_b64_from_ck(field.ck)}{resolved_field.identity_key}"


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
    node_is_attached: Optional[bool] = p_regular(70, require=False, default=None)
    node_types: list["NodeType"] = p_regular(71, array=True)
    # specific node-ish
    block_types: list["BlockType"] = p_regular(80, array=True)
    step_types: list["StepType"] = p_regular(81, array=True)
    file_types: list["FileType"] = p_regular(82, array=True)
    file_formats: list["FileFormat"] = p_regular(83, array=True)
    view_types: list["ViewType"] = p_regular(84, array=True)


constraint = TypeConstraint


@object_()
class TypeInfoBase(BuiltinObject):
    """
    A type is a kind of Value that can go somewhere, often in a place described by a Field.

    A type one of these TypeKinds:
       1. Primitive (= column type, value is scalar, like int32, string, bool, datetime, ...)
          [primitive_type] | [base_type = Block aliased to primitive_type]
       2. Node (value is NodeReference, like Package, Block, Field, Record, Run, Signal, ...)
          [bench_type~NodeType | None]
       3. Struct (value is 'robust json', like Expression, File, Path, Text, Code, ...)
          [bench_type~StructType]
       4. Enum (value is builtin IdEnum, like FieldKind, NodeType, BenchType, EnumType, ...)
          [bench_type~EnumType]
       5. Based Node (value is NodeReference that is an 'instance' of the block)
          [bench_type~NodeType & base_type]
           type = Record, base = Block -> values are Records in that database
           type = Run, base = Block -> values are Runs of that block
           type = Field, base = Block -> values are Fields in that block
           type = Signal, base = Block -> values are Signals of that block type
            ...
       6. Object (value is Object value of classy type, like Code inputs, Step outputs, Record value, ...)
          [base_type~Block[is_classy]|Step]
       7. Alias (value is whatever base_type resolves to, must be resolved to pack/unpack)
       8. Literal (only allowable value is the type itself / or some constant value)
       9. Union (type is union of Field children with oneof=self)

    Types may also specify:
       - field zone, narrowing the fields included from the base type (if any)
       - condition which instances must satisfy
       - constraints (simple conditions the value must satisfy)
       - combination flags for arrays, optionals, ...
    """

    # type identity
    kind: TypeKind = p_internal(40)
    primitive_type: Optional[PrimitiveType] = p_regular(41, default=None)
    bench_type: Optional[BenchType] = p_regular(42, default=None)
    base_type: Union["Block", "Step", None] = p_regular(
        43, array=False, require=False, default=None, references=(NodeType.BLOCK, NodeType.STEP)
    )
    if TYPE_CHECKING:
        base_type_id: Optional[UUID] = None
        base_type_ptr: Optional["NodeReference"] = None
    base_field_zone: Optional["FieldZone"] = p_internal(44, require=False, default=None)
    oneof: Union["Field", "Block", None] = p_regular(
        45,
        require=False,
        array=False,
        default=None,
        references=(NodeType.FIELD, NodeType.BLOCK),
        same_bench=True,
    )

    # metadata
    default_packed: Optional[Any] = p_value_packed(50)
    default = p_value_runtime(packed=50, typ=lambda self: cast("TypeInfoBase", self))
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

    # resolved
    _resolved_type: Optional["TypeInfoBase"] = p_runtime(default=None)
    _resolved_identity_key: str | None = p_runtime(default=None)

    _from_property: Optional["Property"] = p_runtime(default=None)

    def __content_str__(self) -> str:
        kind_str = self.kind.bench_name if self.kind else "<no type>"
        if self.base_type is not None:
            if self.bench_type is not None:
                info_str = f"{self.bench_type.bench_name}->{self.base_type.absolute_path}"
            else:
                info_str = f"{kind_str}->{self.base_type.absolute_path}"
        elif self.bench_type is not None:
            info_str = self.bench_type.bench_name
        elif self.primitive_type is not None:
            info_str = self.primitive_type.bench_name
        else:
            info_str = kind_str
        if self.condition is not None:
            info_str += f" [{self.condition}]"

        flags = []
        if self.is_list:
            flags.append("is_list")
        else:
            flags.append("is_scalar")
        if self.is_required:
            flags.append("is_required")
        if self.is_secret:
            flags.append("is_secret")
        if len(flags) > 0:
            info_str += f" ({', '.join(flags)})"
        if self.base_field_zone:
            info_str += f" [{self.base_field_zone.bench_name}]"
        if self.constraint is not None:
            constraint_str = self.constraint.__content_str__()
            if constraint_str:
                info_str += f" [{constraint_str}]"
        if self._from_property:
            info_str += f" from {self._from_property!s}"

        return info_str

    def _resolve_type(self):
        # TODO :Incomplete: proper type resolution (consider multi-step aliases, inheritance, ...)
        #  (also when do we even call this to react to edits? how do we get this into bench-web?)

        # resolve the actual type :TypeResolution
        resolved_type = self
        if self.kind == TypeKind.ALIAS:
            from bench.language import Block

            assert self.base_type_ptr is not None, f"missing base type for alias {self!r}"
            base_type = self.base_type
            if base_type is not None:  # may not be resolved
                if base_type.metatype == NodeType.STEP or (
                    isinstance(base_type, Block) and base_type.type.is_classy
                ):
                    resolved_type = TypeInfo(kind=TypeKind.OBJECT, base_type=base_type)
                elif base_type.metatype == NodeType.BLOCK and base_type.variable_type is not None:
                    resolved_type = base_type.variable_type

        self._resolved_type = resolved_type
        self._resolved_identity_key = encode_type_identity(resolved_type)
        if resolved_type is not None and resolved_type is not self:
            # ensure the resolved type resolves to itself
            resolved_type._resolved_type = resolved_type
            resolved_type._resolved_identity_key = self._resolved_identity_key
        return resolved_type

    def _to_resolved(self) -> "TypeInfoBase":
        if self._resolved_type is None:
            return self._resolve_type()
        else:
            return self._resolved_type

    @override
    def _validate_component(
        self, properties: Collection[Property], invalid: "ValidationHandler"
    ) -> None:
        implied_kind = get_implied_type_kind(self)
        if (
            implied_kind is not None
            and implied_kind != self.kind
            and implied_kind != TypeKind.ALIAS
        ):
            actual_kind = self.kind.name if self.kind else "None"
            invalid(self, f"kind is {actual_kind} but should be {implied_kind.name}", None)
        if self.is_list and not self.supports_list:
            invalid(self, "list type is not supported", None)

    def __call__(self, *args, **kwargs) -> "SomeValue":
        """Converts the given value to this type."""
        # TODO :Cleanup :Architecture: TypeInfo.__call__ feels a lot like coerce_value
        #  But it's not quite the same. Here we want to error if we can't coerce, return full nodes, etc.
        typ = self._to_resolved()
        if typ.kind == TypeKind.PRIMITIVE:
            py_type = PY_TYPE_BY_PRIMITIVE_TYPE.get(cast(PrimitiveType, typ.primitive_type))
            assert py_type is not None, f"{typ!r} does not have a python type"
            return py_type(*args, **kwargs)
        elif typ.kind == TypeKind.BASED_NODE:
            if typ.bench_type == NodeType.FIELD:
                assert typ.base_type is not None, f"missing base type for {typ!r}"
                field = typ.base_type.fields.get(*args, **kwargs)
                if field is None:
                    raise ValueError(f"no field {args!r} in {typ.base_type!r}")
                return field
        elif typ.kind == TypeKind.OBJECT:
            return coerce_value_object_scalar(kwargs, typ, as_packed=True)

        raise ValueError(f"cannot create {self!r} (resolved={typ!r}) directly")

    @property
    def supports_list(self) -> bool:  # :ListableTypes
        """Whether this type supports lists."""
        if self.kind in (  # noqa: SIM114
            TypeKind.NODE,
            TypeKind.BASED_NODE,
            TypeKind.ENUM,
            TypeKind.OBJECT,
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
        assert self._resolved_identity_key is not None, f"unresolved type {self!r}"
        return self._resolved_identity_key

    @property
    def _base_fields(self) -> NodeList["Field"]:
        assert self._resolved_type is not None, f"unresolved type {self!r}"
        assert self._resolved_type.base_type is not None, f"missing base type {self!r}"
        return self._resolved_type.base_type.fields

    @property
    def _fields(self) -> NodeList["Field"] | Sequence["Field"]:
        assert self._resolved_type is not None, f"unresolved type {self!r}"
        if self._resolved_type.base_field_zone is None:
            return self._base_fields
        else:
            return tuple(
                f for f in self._base_fields if f.type == self._resolved_type.base_field_zone
            )

    def _get_field(self, ident: str) -> Optional["Field"]:
        """Resolves a field in this type by an identifier (name or py_name)"""
        for field in self._base_fields:
            if (field.code_name == ident or field.name == ident) and (
                self.base_field_zone is None or field.type == self.base_field_zone
            ):
                return field
        return None

    def _get_field_by_tk(self, tk: str) -> Optional["Field"]:
        """Resolves a field in this type by its tk"""
        for field in self._base_fields:
            if field.tk == tk:
                return field
        return None


@struct_(StructType.TYPE_INFO)
class TypeInfo(Struct, TypeInfoBase):
    """A type in the type system."""

    # redirect so we get TypeInfoBase.__content_str__ (not Struct.__content_str__)
    __content_str__ = TypeInfoBase.__content_str__  # type: ignore

    @staticmethod
    def from_type(
        typ: "TypeIn", constraint: "TypeConstraintIn | TypeConstraint | None" = None
    ) -> "TypeInfo":
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
    "TypeInfoBase",
    "Block",
    "Step",
    "PrimitiveType",
    "BenchType",
    "TypeFormat",
    "FileType",
    "FileFormat",
    Type[Struct],
    Type[Node],
    Type[PrimitiveValue],
]


def to_type_scalar(
    typ: TypeIn, *, as_object: bool = False, zone: FieldZone | None = None
) -> "TypeInfo":
    """Converts a type-like object to a TypeInfo."""
    from bench.language.file import FileFormat, FileType

    if isinstance(typ, TypeInfoBase):
        return cast("TypeInfo", typ)
    elif isinstance(typ, Node) and typ.metatype in (NodeType.BLOCK, NodeType.STEP):
        type_info = cast("Block|Step", typ).to_type(as_object=as_object, zone=zone)
        if type_info is not None:
            assert isinstance(type_info, TypeInfo), f"expected TypeInfo, got {type_info!r}"
            return type_info
    elif isinstance(typ, PrimitiveType):
        return TypeInfo(kind=TypeKind.PRIMITIVE, primitive_type=typ)
    elif isinstance(typ, (NodeType, StructType, EnumType, BenchType)):
        if is_node_type(typ):
            return TypeInfo(kind=TypeKind.NODE, bench_type=typ)
        elif is_struct_type(typ):
            return TypeInfo(kind=TypeKind.STRUCT, bench_type=typ)
        elif is_enum_type(typ):
            return TypeInfo(kind=TypeKind.ENUM, bench_type=typ)
    elif isinstance(typ, TypeFormat):
        return TypeInfo(kind=TypeKind.PRIMITIVE, primitive_type=typ.primitive_type, format=typ)
    elif isinstance(typ, FileType):
        return TypeInfo(
            kind=TypeKind.NODE,
            bench_type=NodeType.FILE,
            constraint=TypeConstraint(file_types=[typ]),
        )
    elif isinstance(typ, FileFormat):
        return TypeInfo(
            kind=TypeKind.NODE,
            bench_type=NodeType.FILE,
            constraint=TypeConstraint(file_formats=[typ]),
        )
    elif isinstance(typ, type):
        primitive_type = PRIMITIVE_TYPE_BY_PY_TYPE.get(typ)
        if primitive_type:
            return TypeInfo(kind=TypeKind.PRIMITIVE, primitive_type=primitive_type)
        bench_type = BENCH_TYPE_BY_CLASS.get(cast(Any, typ))
        if bench_type is not None:
            if is_node_type(bench_type):
                return TypeInfo(kind=TypeKind.NODE, bench_type=bench_type)
            elif is_struct_type(bench_type):
                return TypeInfo(kind=TypeKind.STRUCT, bench_type=bench_type)
            elif is_enum_type(bench_type):
                return TypeInfo(kind=TypeKind.ENUM, bench_type=bench_type)
        elif typ == Node:
            return TypeInfo(kind=TypeKind.NODE)

    raise ValueError(f"unsupported type {typ!r}")


def to_type(
    typ: TypeIn,
    *,
    as_object: bool = False,
    zone: FieldZone | None = None,
    constraint: TypeConstraintIn | TypeConstraint | None = None,
    is_required: bool = False,
    is_list: bool = False,
) -> TypeInfo:
    """Converts a TypeIn into a TypeInfoBase."""
    type_scalar = to_type_scalar(typ, as_object=as_object, zone=zone)
    if isinstance(constraint, TypeConstraintIn):
        constraint = constraint.into()
    type_scalar.constraint = constraint
    type_scalar.is_required = is_required
    type_scalar.is_list = is_list
    return type_scalar


def reverse_type_scalar(typ: TypeInfoBase) -> TypeIn | None:
    """Reverses a TypeInfo into a TypeIn as closely as possible."""
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
            if len(typ.constraint.file_formats) == 1:
                return typ.constraint.file_formats[0]
            elif len(typ.constraint.file_types) == 1:
                return typ.constraint.file_types[0]
        assert typ.bench_type is not None, f"missing bench type for {typ!r}"
        bench_cls = BENCH_CLASS_BY_TYPE[typ.bench_type]
        return bench_cls
    elif typ.kind == TypeKind.BASED_NODE:
        assert typ.base_type is not None, f"missing base type for {typ!r}"
        return typ.base_type
    else:
        return None  # can't reverse


# pyright: reportIncompatibleMethodOverride=false
@local_node_(NodeType.FIELD)
class Field(SourceNode[FieldData], HasNodeBase, TypeInfoBase, _TypeQueryBuilder):
    """
    A used-defined attribute of some value
     (Bench defines Properties for Nodes/Structs, Users define Fields for Values inside those).
    """

    parent: Union["Block", "Step", None] = p_node_parent(4, NodeType.BLOCK, NodeType.STEP)
    type: FieldZone = p_internal(30, default=FieldZone.VARIABLE)
    name: str = p_regular(31, constraint=NAME_CONSTRAINT)
    order_key: str = p_internal(32, default=INTEGER_ZERO)
    text: Optional["Text"] = p_regular(
        33, default=None, require=False, array=False, struct=StructType.TEXT
    )
    icon: Optional["Icon"] = p_regular(34, require=False, array=False, struct=StructType.ICON)

    # type identity
    # ...TypeInfo[40-69]

    # field-only flags
    # is_indexed? # for database fields
    # is_unique? # for database fields

    _introspected_from: Optional[Property] = p_runtime(default=None)

    @final
    def __repr__(self):  # type: ignore we want to override the default repr
        return f"<{self.type.bench_name}Field {self}>"

    def __content_str__(self) -> str:
        if self.type == FieldZone.OPTION:
            return ""  # nothing to show
        else:
            return TypeInfoBase.__content_str__(self)

    def __eq__(self, other):  # type: ignore
        return _TypeQueryBuilder.__eq__(self, other)  # override to avoid recursion

    __hash__ = SourceNode.__hash__  # type: ignore
    # (not entirely sure why we need to override Field.__hash__ but not for any other node, maybe
    #  one of the base structs takes precende for some reason (but SourceNode is first in MRO...))

    def _validate_component(
        self, properties: Collection[Property], invalid: ValidationHandler
    ) -> None:
        if (self.type == FieldZone.OPTION) != (self.kind == TypeKind.LITERAL):
            invalid(self, "option field must be literal", None)

    @property
    def base(self) -> Optional[Node]:
        return self.parent

    @property
    def base_ck(self) -> Optional[UUID]:
        return self.base.ck if self.base is not None else None

    @staticmethod
    def get_base_from_data(data: AnyNodeData) -> Optional[NodeReferenceData]:
        return (cast(FieldData, data)).parent_ptr

    key = SourceNode.tk

    @property
    def storage_key(self) -> str:
        return encode_storage_key(self)

    @staticmethod
    def new(
        name: str,
        typ: TypeIn,
        constraint: TypeConstraintIn | TypeConstraint | None = None,
        **kwargs,
    ) -> "Field":
        typ = to_type_scalar(typ)
        for prop in TypeInfoBase.__declared_properties__.values():
            if prop.name not in kwargs:
                kwargs[prop.name] = getattr(typ, prop.name)
        if constraint is not None:
            if isinstance(constraint, TypeConstraintIn):
                constraint = constraint.into()
            kwargs["constraint"] = constraint
        field = Field(name=name, **kwargs)
        return field

    @staticmethod
    def option(name: str, **kwargs) -> "Field":
        return Field(kind=TypeKind.LITERAL, type=FieldZone.OPTION, name=name, **kwargs)

    @staticmethod
    def variable(
        name: str,
        typ: TypeIn,
        constraint: TypeConstraintIn | TypeConstraint | None = None,
        **kwargs,
    ) -> "Field":
        return Field.new(name, typ, zone=FieldZone.VARIABLE, constraint=constraint, **kwargs)

    @staticmethod
    def member(
        name: str,
        typ: TypeIn,
        constraint: TypeConstraintIn | TypeConstraint | None = None,
        **kwargs,
    ) -> "Field":
        return Field.new(name, typ, zone=FieldZone.MEMBER, constraint=constraint, **kwargs)

    @staticmethod
    def input(
        name: str,
        typ: TypeIn,
        constraint: TypeConstraintIn | TypeConstraint | None = None,
        **kwargs,
    ) -> "Field":
        return Field.new(name, typ, zone=FieldZone.INPUT, constraint=constraint, **kwargs)

    @staticmethod
    def output(
        name: str,
        typ: TypeIn,
        constraint: TypeConstraintIn | TypeConstraint | None = None,
        **kwargs,
    ) -> "Field":
        return Field.new(name, typ, zone=FieldZone.OUTPUT, constraint=constraint, **kwargs)
