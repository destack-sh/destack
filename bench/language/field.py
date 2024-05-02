import typing
from datetime import datetime, timedelta
from typing import TYPE_CHECKING, Any, Collection, Optional, Union, cast
from uuid import UUID

import structlog

from bench.language.const import (
    TK_LENGTH_B64,
    BenchType,
    FieldZone,
    FormatHint,
    NodeType,
    NodeVisibility,
    StructType,
    TypeKind,
    is_enum_type,
    is_node_type,
    is_struct_type,
)
from bench.language.expression import NodeReference, _TypeQueryBuilder
from bench.language.graph import NodeList
from bench.language.node import (
    Node,
    get_tk_b64_from_ptr,
    node,
    pad_ck_from_tk_b64,
    struct,
    struct_component,
)
from bench.language.notice import Notice
from bench.language.property import (
    Property,
    p_internal,
    p_node_child,
    p_node_parent,
    p_regular,
    p_runtime,
    p_value_packed,
    p_value_runtime,
)
from bench.language.validation import ValidationHandler, validate_name
from bench.language.value import HasValues
from bench.proto.wire import FieldData
from bench.sql.core import PrimitiveType
from bench.utils.casing import IdentifierType
from bench.utils.fractional import INTEGER_ZERO
from bench.utils.func import decode_b64vlq, encode_b64vlq

if typing.TYPE_CHECKING:
    from bench.language import Block, Expression, Icon, Step, Text
    from bench.language.notice import NoticeHandler

# pyright: reportIncompatibleVariableOverride=false,reportIncompatibleMethodOverride=false

logger = structlog.get_logger(__name__)

PYTON_TYPE_BY_PRIMITIVE_TYPE: dict[PrimitiveType, type] = {
    PrimitiveType.BOOLEAN: bool,
    PrimitiveType.INT16: int,
    PrimitiveType.INT32: int,
    PrimitiveType.INT64: int,
    PrimitiveType.FLOAT32: float,
    PrimitiveType.FLOAT64: float,
    PrimitiveType.STRING: str,
    PrimitiveType.BYTES: bytes,
    PrimitiveType.UUID: UUID,
    PrimitiveType.DATETIME: datetime,
    PrimitiveType.INTERVAL: timedelta,
}


LETTER_BY_TYPE_KIND: dict[TypeKind, str] = {
    TypeKind.PRIMITIVE: "p",
    TypeKind.STRUCT: "s",
    TypeKind.NODE: "n",
    TypeKind.ENUM: "e",
    TypeKind.BASED_NODE: "b",
    TypeKind.OBJECT: "o",
}
TYPE_KIND_BY_LETTER: dict[str, TypeKind] = {v: k for k, v in LETTER_BY_TYPE_KIND.items()}


def get_implied_type_kind(typ: "TypeInfoBase") -> TypeKind | None:
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
        base_type = typ.base_type
        if base_type is not None:
            if base_type.metatype == NodeType.STEP or cast("Block", base_type).type.is_classy:
                return TypeKind.OBJECT
            else:
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
        assert typ.primitive_type is not None
        value = encode_b64vlq(typ.primitive_type.id)
    elif typ.kind == TypeKind.NODE or typ.kind == TypeKind.STRUCT or typ.kind == TypeKind.ENUM:
        assert typ.bench_type is not None
        value = encode_b64vlq(typ.bench_type.id)
    elif typ.kind == TypeKind.BASED_NODE:
        assert typ.base_type_ptr is not None
        assert typ.bench_type is not None
        value = f"{get_tk_b64_from_ptr(typ.base_type_ptr)}{encode_b64vlq(typ.bench_type.id)}"
    elif typ.kind == TypeKind.OBJECT:
        assert typ.base_type_ptr is not None
        value = get_tk_b64_from_ptr(typ.base_type_ptr)
    else:
        return None

    prefix: str
    if typ.is_list:
        prefix = LETTER_BY_TYPE_KIND[typ.kind].upper()
    else:
        prefix = LETTER_BY_TYPE_KIND[typ.kind]
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
        return TypeInfo(primitive_type=primitive_type, is_list=is_list, is_secret=is_secret)
    elif (
        kind == TypeKind.NODE.value or kind == TypeKind.STRUCT.value or kind == TypeKind.ENUM.value
    ):
        bench_type = BenchType(decode_b64vlq(value))  # type: ignore
        return TypeInfo(bench_type=bench_type, is_list=is_list, is_secret=is_secret)
    elif kind == TypeKind.BASED_NODE.value:
        base_type_ptr = NodeReference(
            type=NodeType.BLOCK, ck=pad_ck_from_tk_b64(value[:TK_LENGTH_B64])
        )
        bench_type = BenchType(decode_b64vlq(value[TK_LENGTH_B64:]))  # type: ignore
        return TypeInfo(
            base_type_ptr=base_type_ptr, bench_type=bench_type, is_list=is_list, is_secret=is_secret
        )
    elif kind == TypeKind.OBJECT.value:
        base_type_ptr = NodeReference(
            type=NodeType.BLOCK, ck=pad_ck_from_tk_b64(value[:TK_LENGTH_B64])
        )
        return TypeInfo(base_type_ptr=base_type_ptr, is_list=is_list, is_secret=is_secret)

    raise ValueError(f"unsupported type kind {kind}")


@struct_component()
class TypeInfoBase(HasValues):
    """
    A type is a kind of value that can go somewhere, typically in a place designated by a Field.

    A type one of these TypeKinds:
       1. Primitive (= column type, value is scalar, like int32, string, bool, datetime, ...)
          [primitive_type] | [base_type = Block aliased to primitive_type]
       2. Node (value is NodeReference, like Package, Block, Field, Record, Run, Signal, ...)
          [bench_type~NodeType]
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
       6. Value (value is Value of classy type, like Code inputs, Step outputs, Record value, ...)
          [base_type~Block[is_classy]|Step]
       7. Alias (value is whatever base_type resolves to, must be resolved to pack/unpack)

    Types may also specify:
       - field zone, narrowing the fields included from the base type (if any)
       - format hint (which may impact the unpacked representation, like for Image)
       - condition which instances must satisfy
       - combination flags for arrays, optionals, ...

    Type checking is done in ./value.py. Checking certain invariants requires querying the graph.
    """

    # type identity (if unset this isn't a valid type (used for Field Options))
    kind: Optional[TypeKind] = p_internal(40, require=False, default=None)
    primitive_type: Optional[PrimitiveType] = p_regular(41, default=None)
    bench_type: Optional[BenchType] = p_regular(42, default=None)
    base_type: Union["Block", "Step", None] = p_regular(
        43, array=False, require=False, default=None, references=(NodeType.BLOCK, NodeType.STEP)
    )
    if TYPE_CHECKING:
        base_type_id: Optional[UUID] = None
        base_type_ptr: Optional["NodeReference"] = None
    base_field_zone: Optional["FieldZone"] = p_internal(44, require=False, default=None)

    # + bonus info/constraints
    visibility: Optional[NodeVisibility] = p_regular(50, default=None)
    format_hint: Optional[FormatHint] = p_regular(51, default=None)
    condition: Optional["Expression"] = p_regular(
        52, require=False, array=False, default=None, struct=StructType.EXPRESSION
    )
    default_packed: Optional[Any] = p_value_packed(53)
    default = p_value_runtime(packed=53)

    # flags
    is_list: bool = p_regular(60, default=False)
    is_secret: bool = p_regular(61, default=False)
    is_required: bool = p_regular(62, default=False)

    # resolved
    _resolved_type: Optional["TypeInfoBase"] = p_runtime(default=None)
    _resolved_identity_key: str | None = p_runtime(default=None)

    def __content_str__(self) -> str:
        kind_str = self.kind.bench_name if self.kind else "<no type>"
        if self.base_type is not None:
            info_str = f"{kind_str}->{self.base_type.absolute_path}"
        elif self.bench_type is not None:
            info_str = self.bench_type.bench_name
        elif self.primitive_type is not None:
            info_str = self.primitive_type.name
        else:
            info_str = kind_str
        if self.format_hint:
            info_str += f" as {self.format_hint}"
        if self.condition is not None:
            info_str += f" [{self.condition}]"

        flags = tuple(f for f in ("is_list", "is_required", "is_secret") if getattr(self, f))
        if flags:
            info_str += f" ({', '.join(flags)})"
        return info_str

    def _interp_component(self, scope: Optional["Node"], notice: "NoticeHandler"):
        # TODO :Incomplete: proper type resolution (consider multi-step aliases, inheritance, ...)
        if self.kind == TypeKind.ALIAS:
            assert self.base_type is not None, f"missing base type for alias {self!r}"
            if (
                self.base_type.metatype == NodeType.STEP
                or cast("Block", self.base_type).type.is_classy
            ):
                resolved_type = TypeInfo(kind=TypeKind.OBJECT, base_type=self.base_type)
            elif (
                self.base_type.metatype == NodeType.BLOCK
                and cast("Block", self.base_type).builtin_base
            ):
                resolved_type = cast("Block", self.base_type).builtin_base
            else:
                resolved_type = None
        else:
            resolved_type = self
        self._do_resolve_to(resolved_type)

    def _validate_component(
        self, properties: Collection[Property], invalid: "ValidationHandler"
    ) -> None:
        implied_kind = get_implied_type_kind(self)
        if implied_kind is not None and implied_kind != self.kind:
            actual_kind = self.kind.name if self.kind else "None"
            invalid(self, f"implied kind {implied_kind.name} does not match {actual_kind}", None)

    @property
    def identity_key(self) -> str:
        """The identity of this type for packing."""
        assert self._resolved_identity_key is not None, f"unresolved type {self!r}"
        return self._resolved_identity_key

    def _do_resolve_to(self, typ: Optional["TypeInfoBase"]) -> None:
        self._resolved_type = typ
        self._resolved_identity_key = encode_type_identity(typ) if typ else None
        if typ is not None and typ is not self:
            # ensure the resolved type resolves to itself
            typ._resolved_type = typ
            typ._resolved_identity_key = self._resolved_identity_key

    def _as_resolved(self) -> "TypeInfoBase":
        assert self._resolved_type is not None, f"unresolved type {self!r}"
        assert self._resolved_type.kind is not None, f"missing type identity {self!r}"
        return self._resolved_type

    @property
    def _base_fields(self) -> NodeList["Field"]:
        assert self._resolved_type is not None, f"unresolved type {self!r}"
        assert self._resolved_type.base_type is not None, f"missing base type {self!r}"
        return self._resolved_type.base_type.fields

    def _get_field(self, ident: str) -> Optional["Field"]:
        """Resolves a field in this type by an identifier (name or py_ident)"""
        for field in self._base_fields:
            if field.py_ident == ident or field.name == ident:
                return field
        return None


@struct(StructType.TYPE_INFO)
class TypeInfo(TypeInfoBase):
    pass


@node(NodeType.FIELD)
class Field(Node[FieldData], TypeInfoBase, _TypeQueryBuilder):
    """
    A used-defined attribute of some value
     (Bench defines Properties for Nodes/Structs, Users define Fields for Values inside those).
    """

    parent: Union["Block", "Step", None] = p_node_parent(4, NodeType.BLOCK, NodeType.STEP)
    name: str = p_regular(30, validate=validate_name)
    order_key: str = p_internal(31, default=INTEGER_ZERO)
    zone: FieldZone = p_internal(32, default=FieldZone.VARIABLE)
    text: Optional["Text"] = p_regular(
        33, default=None, require=False, array=False, struct=StructType.TEXT
    )
    icon: Optional["Icon"] = p_regular(34, require=False, array=False, struct=StructType.ICON)
    value_packed: Any | None = p_value_packed(35)
    value = p_value_runtime(35)

    # type identity
    # ...TypeInfo

    # field-only flags
    # is_indexed: bool = ... # for database fields
    # is_unique: bool = ... # for database fields

    notices: NodeList["Notice"] = p_node_child(NodeType.NOTICE)

    _introspected_from: Optional[Property] = p_runtime(default=None)

    def __content_str__(self) -> str:
        if self.base_type is not None:
            info_str = self.base_type.absolute_path
        elif self.bench_type is not None:
            info_str = self.bench_type.bench_name
        elif self.primitive_type is not None:
            info_str = self.primitive_type.name
        else:
            info_str = "<no type>"
        if self.format_hint:
            info_str += f" as {self.format_hint}"
        if self.condition is not None:
            info_str += f" [{self.condition!r}]"

        flags = tuple(f for f in ("is_list", "is_required", "is_secret") if getattr(self, f))
        if flags:
            info_str += f" ({', '.join(flags)})"
        return info_str

    def __eq__(self, other):  # type: ignore
        return _TypeQueryBuilder.__eq__(self, other)  # override to avoid recursion

    def _validate_component(
        self, properties: Collection[Property], invalid: ValidationHandler
    ) -> None:
        if self.zone != FieldZone.OPTION and self.kind is None:
            invalid(self, "missing type identity", None)

    @property
    def identifier_type(self):
        if self.kind == FieldZone.OPTION:
            return IdentifierType.CONSTANT
        else:
            return IdentifierType.PROPERTY

    @property
    def storage_key(self) -> str:
        return f"{self.tk}-{self.identity_key}"
