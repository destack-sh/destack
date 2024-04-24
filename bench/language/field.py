import typing
from typing import Any, Collection, Optional, Union

import structlog

from bench.language.const import (
    BenchError,
    BenchType,
    BlockType,
    EnumType,
    FormatHint,
    NodeType,
    NodeVisibility,
    StructType,
    enum_,
)
from bench.language.expression import _TypeQueryBuilder
from bench.language.node import Node, NodeList, node, struct, struct_component
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
from bench.sql.core import PrimitiveType
from bench.utils.casing import IdentifierType
from bench.utils.fractional import INTEGER_ZERO
from bench.utils.func import IdEnum

if typing.TYPE_CHECKING:
    from bench.language import Block, Expression, Icon, Step, Text
    from bench.language.notice import NoticeHandler

logger = structlog.get_logger(__name__)


class TypeError(BenchError, TypeError):
    def __init__(
        self,
        value: Any,
        expected: "TypeInfo",
        message: str = None,
        suberrors: list["TypeError"] = None,
    ):
        value_str = repr(value)
        max_value_str_len = 300
        if len(value_str) > max_value_str_len:
            value_str = value_str[: max_value_str_len - 100] + "..." + value_str[-100:]

        if isinstance(expected, Field) and not expected.fields:
            expected_str = f"field '{expected.py_ident}' ({expected._type_str})"
        else:
            expected_fields_str = ", ".join(
                f"'{f.py_ident}' ({f._type_str})" for f in expected.fields
            )
            expected_str = f"fields {expected_fields_str or '<empty>'} from {expected!r}"

        super().__init__(
            f"{message or 'type mismatch'}: expected {expected_str}, got {value_str} ({type(value).__name__})"
        )
        self.value = value
        self.expected = expected
        self.message = message
        self.suberrors = suberrors or []


def encode_type_info_identity(type: "TypeInfoBase") -> str:
    """Encodes the type info into a key for storage & implicit typing. :TypeInfoEncoding"""
    # assert type.primitive_type is not None, f"no column type in {type!r}"
    # key_parts = [str(type.primitive_type.id)]
    # if type.is_list:
    #     key_parts.append("a")
    # if type.is_secret:
    #     key_parts.append("s")
    # if type.length:
    #     key_parts.append(f"l{type.length}")
    # if type.precision:
    #     key_parts.append(f"p{type.precision}")
    # if type.scale:
    #     key_parts.append(f"s{type.scale}")
    # if type.base_type:
    #     key_parts.append(type.base_type.sk)
    # key = "".join(key_parts)
    # return key
    raise NotImplementedError


def decode_type_info_identity(identity_key: str) -> "TypeInfoBase":
    """Decodes the type-related info back from the identity key. :TypeInfoEncoding"""
    raise NotImplementedError


@struct_component
class TypeInfoBase(HasValues):
    """
    A type is a kind of value that can go somewhere, typically in place of a Field.

    A type is either:
       1. primitive type (= column type, value is scalar, like int32, string, bool, datetime, ...)
          [primitive_type] | [base_type = Block aliased to primitive_type]
       2. struct type (value is 'robust json', like Expression, File, BenchPath, RichText, ...)
          [struct_type] | [base_type is newtype with object_type]
       3. node type (value is NodeReference, like Package, Block, Field, Record, Run, Signal, ...)
          [node_type] | [base_type = Block aliased to object_type]
       4. reference to a block (value is NodeReference that is an 'instance' of the block)
          [node_type & base_type = Block]
           type = Record, base = DatabaseBlock -> values must be Records in that database
           type = Run, base = Block -> values must be Runs of that block
           type = Field, base = Block -> values must be a Field in that block
           type = Signal, base = Block -> values must be Signals of that block type
           type = Block, base = Block -> values must be Blocks conforming to that block protocol
           type = Block, base = None -> values must be instances of the resolved type
            ...

    Types may also specify:
       - a format hint (which may impact the unpacked/instantiated Python representation, like for Image)
       - an additional condition instances must satisfy
       - combination flags for arrays, optionals, ...

    Type checking is done in ./value.py. You'll note that we can only check some things without querying.
    """

    # type identity (must set at least one of these)
    primitive_type: Optional[PrimitiveType] = p_regular(40, default=None)
    bench_type: Optional[BenchType] = p_regular(41, default=None)
    base_type: Optional["Block"] = p_regular(
        42, array=False, require=False, default=None, references=NodeType.BLOCK
    )

    # + bonus info/constraints
    visibility: Optional[NodeVisibility] = p_regular(50, default=None)
    format_hint: Optional[FormatHint] = p_regular(51, default=None)
    condition: Optional["Expression"] = p_regular(
        52, require=False, array=False, default=None, struct=StructType.EXPRESSION
    )
    length: Optional[int] = p_regular(53, require=False, default=None)
    precision: Optional[int] = p_regular(54, require=False, default=None)
    scale: Optional[int] = p_regular(55, require=False, default=None)
    # default for this type
    default_packed: Optional[Any] = p_value_packed(58)
    default = p_value_runtime(packed=58)

    # flags
    is_list: bool = p_regular(60, default=False)
    is_required: bool = p_regular(61, default=False)
    is_secret: bool = p_regular(62, default=False)
    # is_instance to disambiguate?

    # separate _fields for restricting base type to a subset of fields? (e.g., only inputs)
    _fields: tuple["Field", ...] | None = p_runtime(default=None)
    _resolved_type: Optional["TypeInfo"] = p_runtime(default=None)

    def __content_str__(self) -> str:
        if self.base_type is not None:
            info_str = self.base_type.absolute_path
        elif self.node_type is not None:
            info_str = self.node_type.bench_name
        elif self.struct_type is not None:
            info_str = self.struct_type.bench_name
        elif self.primitive_type is not None:
            info_str = self.primitive_type.name
        else:
            info_str = "<no type>"
        if self.format_hint:
            info_str += f" as {self.format_hint}"
        if self.condition:
            info_str += f" [{self.condition}]"

        flags = tuple(f for f in ("is_list", "is_required", "is_secret") if getattr(self, f))
        if flags:
            info_str += f" ({', '.join(flags)})"
        return info_str

    def _interp_inner(self, scope: "Node", on_notice: "NoticeHandler"):
        if self.base_type is not None and self.base_type.type == BlockType.ALIAS:
            raise NotImplementedError(f"aliases not yet supported for {self!r}")
        else:
            self._resolved_type = self

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
            on_invalid(self, "missing type identity")

    @property
    def identity_key(self) -> str:
        """
        The identity of this type for storing. Different keys mean you won't get the value back out.
        """
        return encode_type_info_identity(self)

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
class Field(Node, TypeInfoBase, _TypeQueryBuilder):
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
        if self.condition:
            info_str += f" [{self.condition}]"

        flags = tuple(f for f in ("is_list", "is_required", "is_secret") if getattr(self, f))
        if flags:
            info_str += f" ({', '.join(flags)})"
        return info_str

    def _as_type(self) -> "TypeInfo":
        return self._resolved_type

    def __eq__(self, other):
        return _TypeQueryBuilder.__eq__(self, other)  # override to avoid recursion

    @property
    def identifier_type(self):
        if self.kind == FieldKind.OPTION:
            return IdentifierType.CONSTANT
        else:
            return IdentifierType.PROPERTY

    @property
    def storage_key(self) -> str:
        return f"{self.sk}-{self.resolved_type.identity_key}"
