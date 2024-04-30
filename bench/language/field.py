import enum
import typing
from typing import TYPE_CHECKING, Any, Collection, Optional, Union, cast
from uuid import UUID

import structlog

from bench.language.const import (
    TK_LENGTH_B64,
    BenchError,
    BenchType,
    BlockType,
    EnumType,
    FormatHint,
    NodeType,
    NodeVisibility,
    StructType,
    enum_,
    is_enum_type,
    is_node_type,
    is_struct_type,
)
from bench.language.expression import NodeReference, _TypeQueryBuilder
from bench.language.node import (
    Node,
    NodeList,
    get_tk_b64_from_ptr,
    node,
    pad_ck_from_tk_b64,
    struct,
    struct_component,
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
from bench.language.validation import ValidationHandler, validate_name
from bench.language.value import HasValues
from bench.proto.wire import FieldData
from bench.sql.core import PrimitiveType
from bench.utils.casing import IdentifierType
from bench.utils.fractional import INTEGER_ZERO
from bench.utils.func import IdEnum, decode_b64vlq, encode_b64vlq

if typing.TYPE_CHECKING:
    from bench.language import Block, Expression, Icon, Step, Text
    from bench.language.notice import NoticeHandler

# pyright: reportIncompatibleVariableOverride=false,reportIncompatibleMethodOverride=false

logger = structlog.get_logger(__name__)


class TypeError(BenchError, TypeError):
    def __init__(
        self,
        value: Any,
        expected: "TypeInfo",
        message: str | None = None,
        suberrors: list["TypeError"] | None = None,
    ):
        value_str = repr(value)
        max_value_str_len = 300
        if len(value_str) > max_value_str_len:
            value_str = value_str[: max_value_str_len - 100] + "..." + value_str[-100:]
        super().__init__(
            f"{message or 'type mismatch'}: expected {expected!r}, got {value_str} ({type(value).__name__})"
        )
        self.value = value
        self.expected = expected
        self.message = message
        self.suberrors = suberrors or []


class TypeKind(enum.StrEnum):
    """The 'kind' of a Type. Only for encoding for now. :TypeInfoEncoding"""

    PRIMITIVE = "p"
    STRUCT = "s"
    NODE = "n"
    ENUM = "e"
    BASE = "b"
    ALIAS = "a"


def encode_type_identity(type: "TypeInfoBase") -> str:
    """
    Encodes the type identity into a key for storage & implicit typing.
    Format is <kind>[id] (with id encoded as base64).
    :TypeInfoEncoding
    """
    kind = None
    value = None
    if type.primitive_type:
        kind = TypeKind.PRIMITIVE.value
        value = encode_b64vlq(type.primitive_type.id)
    elif type.bench_type:
        if is_node_type(type.bench_type):
            if not type.base_type_ptr:
                kind = TypeKind.NODE.value
                value = encode_b64vlq(type.bench_type.id)
            else:
                kind = TypeKind.BASE.value
                value = (
                    f"{get_tk_b64_from_ptr(type.base_type_ptr)}{encode_b64vlq(type.bench_type.id)}"
                )
        elif is_struct_type(type.bench_type):
            kind = TypeKind.STRUCT.value
            value = encode_b64vlq(type.bench_type.id)
        elif is_enum_type(type.bench_type):
            kind = TypeKind.ENUM.value
            value = encode_b64vlq(type.bench_type.id)
    elif type.base_type_ptr:
        kind = TypeKind.ALIAS.value
        value = get_tk_b64_from_ptr(type.base_type_ptr)
    if kind is None:
        raise ValueError(f"unsupported type {type!r}")

    if type.is_list:
        prefix = kind.upper()
    else:
        prefix = kind
    if type.is_secret:
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
        kind = key[0].lower()
    else:
        is_list = False
        kind = key[0]
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
    elif kind == TypeKind.BASE.value:
        base_type_ptr = NodeReference(
            type=NodeType.BLOCK, ck=pad_ck_from_tk_b64(value[:TK_LENGTH_B64])
        )
        bench_type = BenchType(decode_b64vlq(value[TK_LENGTH_B64:]))  # type: ignore
        return TypeInfo(
            base_type_ptr=base_type_ptr, bench_type=bench_type, is_list=is_list, is_secret=is_secret
        )
    elif kind == TypeKind.ALIAS.value:
        base_type_ptr = NodeReference(
            type=NodeType.BLOCK, ck=pad_ck_from_tk_b64(value[:TK_LENGTH_B64])
        )
        return TypeInfo(base_type_ptr=base_type_ptr, is_list=is_list, is_secret=is_secret)

    raise ValueError(f"unsupported type kind {kind}")


@struct_component()
class TypeInfoBase(HasValues):
    """
    A type is a kind of value that can go somewhere, typically in a place designated by a Field.

    A type is either:
       1. primitive type (= column type, value is scalar, like int32, string, bool, datetime, ...)
          [primitive_type] | [base_type = Block aliased to primitive_type]
       2. node type (value is NodeReference, like Package, Block, Field, Record, Run, Signal, ...)
          [bench_type~NodeType]
       3. struct type (value is 'robust json', like Expression, File, Path, Text, Code, ...)
          [bench_type~StructType]
       4. enum type (value is builtin IdEnum, like FieldKind, NodeType, BenchType, EnumType, ...)
          [bench_type~EnumType]
       5. node type + base block (value is NodeReference that is an 'instance' of the block)
          [bench_type~NodeType & base_type]
           type = Record, base = Block -> values are Records in that database
           type = Run, base = Block -> values are Runs of that block
           type = Field, base = Block -> values are Fields in that block
           type = Signal, base = Block -> values are Signals of that block type
           type = Block, base = Block -> values are Blocks conforming to that block protocol
            ...
       6. base block (value is whatever that resolves to)

    Types may also specify:
       - a format hint (which may impact the unpacked representation, like for Image)
       - an additional condition instances must satisfy
       - combination flags for arrays, optionals, ...

    Type checking is done in ./value.py. Checking certain invariants requires querying the graph.
    """

    # type identity (must set at least one of these)
    primitive_type: Optional[PrimitiveType] = p_regular(40, default=None)
    bench_type: Optional[BenchType] = p_regular(41, default=None)
    base_type: Optional["Block"] = p_regular(
        42, array=False, require=False, default=None, references=NodeType.BLOCK
    )
    if TYPE_CHECKING:
        base_type_id: Optional[UUID] = None
        base_type_ptr: Optional["NodeReference"] = None

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
    # is_instance to disambiguate?

    # separate _fields for restricting base type to a subset of fields? (e.g., only inputs)
    _fields: tuple["Field", ...] | None = p_runtime(default=None)
    _resolved_type: Optional["TypeInfo"] = p_runtime(default=None)

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
            info_str += f" [{self.condition}]"

        flags = tuple(f for f in ("is_list", "is_required", "is_secret") if getattr(self, f))
        if flags:
            info_str += f" ({', '.join(flags)})"
        return info_str

    def _interp_inner(self, scope: Optional["Node"], on_notice: "NoticeHandler"):
        if self.base_type is not None and self.base_type.type == BlockType.ALIAS:
            raise NotImplementedError(f"aliases not yet supported for {self!r}")
        else:
            self._resolved_type = cast("TypeInfo", self)  # harmless lie (types are equivalent)

    @property
    def resolved_type(self) -> "TypeInfo":
        """
        The complete type of this field including any bases.
        The resolved type is generally the same except when we have aliases.
        """
        assert self._resolved_type is not None, f"resolved type not ready in {self!r}"
        return self._resolved_type

    def _validate_inner(
        self, properties: Collection[Property], on_invalid: "ValidationHandler"
    ) -> None:
        if self.primitive_type is None and self.bench_type is None and self.base_type_ptr is None:
            on_invalid(self, "missing type identity", None, None)

    @property
    def identity_key(self) -> str:
        """The identity of this type for packing."""
        return encode_type_identity(self)

    @property
    def is_nested(self) -> bool:
        """Whether the value of this type has fields."""
        return self._fields is not None or self.base_type is not None

    @property
    def fields(self) -> NodeList["Field"] | tuple["Field", ...] | None:
        if self._fields is not None:
            return self._fields
        elif self.base_type:
            return self.base_type.fields
        else:
            return None


@struct(StructType.TYPE_INFO)
class TypeInfo(TypeInfoBase):
    pass


@enum_(EnumType.FIELD_KIND)
class FieldKind(IdEnum):
    VARIABLE = 1
    MEMBER = 2
    INPUT = 3
    OUTPUT = 4
    OPTION = 5


@node(NodeType.FIELD)
class Field(Node[FieldData], TypeInfoBase, _TypeQueryBuilder):
    """
    A used-defined attribute of some value
     (Bench defines Properties for Nodes/Structs, Users define Fields for Values inside those).
    """

    parent: Union["Block", "Step", None] = p_node_parent(4, NodeType.BLOCK, NodeType.STEP)
    name: str | None = p_regular(30, default=None, validate=validate_name)
    order_key: str = p_internal(31, default=INTEGER_ZERO)
    text: Optional["Text"] = p_regular(
        33, default=None, require=False, array=False, struct=StructType.TEXT
    )
    icon: Optional["Icon"] = p_regular(34, require=False, array=False, struct=StructType.ICON)
    value_packed: Any | None = p_value_packed(35)
    value = p_value_runtime(35)
    kind: FieldKind = p_internal(36, default=FieldKind.VARIABLE)

    # type identity
    # ...TypeInfo

    # field-only flags
    # is_indexed: bool = ... # for database fields
    # is_unique: bool = ... # for database fields
    # is_context: bool = ... # for variable fields (contribute to Context)

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

    @property
    def as_type(self) -> "TypeInfo":
        assert self._resolved_type, f"{self!r} is not resolved"
        return self._resolved_type

    def __eq__(self, other):  # type: ignore
        return _TypeQueryBuilder.__eq__(self, other)  # override to avoid recursion

    @property
    def identifier_type(self):
        if self.kind == FieldKind.OPTION:
            return IdentifierType.CONSTANT
        else:
            return IdentifierType.PROPERTY

    @property
    def storage_key(self) -> str:
        return f"{self.tk}-{self.resolved_type.identity_key}"
