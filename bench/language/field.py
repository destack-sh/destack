import typing
from typing import Any, Optional, Union

import structlog

from bench.language.const import (
    NODE_TYPES,
    STRUCT_TYPES,
    BenchError,
    BenchType,
    BlockType,
    FormatHint,
    NodeType,
    NodeVisibility,
    StructType,
    new_dynamic_node_key,
)
from bench.language.expression import _TypeExpressionBase
from bench.language.node import (
    Node,
    NodeList,
    node,
    struct,
    struct_component,
)
from bench.language.property import (
    Property,
    p_runtime,
    p_node_parent,
    p_value_runtime,
    p_value_packed,
    p_regular,
    p_internal,
)
from bench.language.validation import validate_name
from bench.language.value import HasValues
from bench.sql.core import PrimitiveType
from bench.utils.casing import IdentifierType

if typing.TYPE_CHECKING:
    from bench.language import Block, Expression, Text
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


@struct_component
class TypeInfoBase(HasValues):
    """
    A type is a kind of value that can go somewhere, typically a field.

    A type is either:
       1. primitive type (= column type, value is scalar, like int32, string, bool, datetime, ...)
          [primitive_type] | [base_type = Block aliased to primitive_type]
       2. struct type (value is 'robust json', like Expression, File, BenchPath, RichText, ...)
          [bench_type = StructType] | [base_type is newtype with bench_type]
       3. node type (value is NodeReference, like Package, Block, Field, Record, Run, Signal, ...)
          [bench_type = NodeType] | [base_type = Block aliased to bench_type]
       4. reference to a block (value is NodeReference that is an 'instance' of the block)
          [bench_type = NodeType & base_type = Block]
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
    visibility: NodeVisibility = p_regular(43, default=NodeVisibility.ALL)
    format_hint: Optional[FormatHint] = p_regular(44, default=None)
    condition: Optional["Expression"] = p_regular(
        45, require=False, array=False, default=None, struct=StructType.EXPRESSION
    )
    length: Optional[int] = p_regular(46, require=False, default=None)
    precision: Optional[int] = p_regular(47, require=False, default=None)
    scale: Optional[int] = p_regular(48, require=False, default=None)
    # default for this type :GeneralizeHasValue
    default_packed: Optional[Any] = p_value_packed(49)
    default = p_value_runtime(49)

    # flags
    is_array: bool = p_regular(50, default=False)
    is_required: bool = p_regular(51, default=False)
    is_secret: bool = p_regular(52, default=False)
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
        if self.condition:
            info_str += f" [{self.condition}]"

        flags = tuple(f for f in ("is_array", "is_required", "is_secret") if getattr(self, f))
        if flags:
            info_str += f" ({', '.join(flags)})"
        return info_str

    def _interp_inner(self, scope: "Node", on_notice: "NoticeHandler"):
        if self.base_type is not None and self.base_type.type == BlockType.ALIAS:
            raise NotImplementedError(f"aliases not yet supported for {self!r}")
        else:
            self._resolved_type = self

    def extend(self, **kwargs) -> "TypeInfo":
        """Returns a new type that is the same as this one, but with the given properties overridden."""
        combined_kwargs = {
            prop.name: getattr(self, prop.name) for prop in TypeInfo.__runtime_properties__.values()
        }
        for k, v in kwargs.items():
            combined_kwargs[k] = v
        return TypeInfo(**combined_kwargs)

    @property
    def resolved_type(self) -> "TypeInfo":
        """
        The complete type of this field including any bases.
        The resolved type is generally the same except when we have aliases.
        """
        assert self._resolved_type is not None, f"resolved type not ready in {self!r}"
        return self._resolved_type

    @property
    def identity_key(self) -> str:
        """The identity of this type for storing. Different keys mean you won't get the value back out."""
        assert self.primitive_type is not None, f"no column type in {self!r}"
        key = str(self.primitive_type.id)
        if self.is_array:
            key += "a"
        if self.is_secret:
            key += "e"
        if self.length:
            key += f"l{self.length}"
        if self.precision:
            key += f"p{self.precision}"
        if self.scale:
            key += f"s{self.scale}"
        if self.base_type:
            key += "-" + self.base_type.dynamic_key
        return key

    @property
    def base_type_type(self) -> Optional[BlockType]:
        if self.base_type:
            return self.base_type.type
        else:
            return None

    @property
    def is_reference(self) -> bool:
        """Whether this is a"""
        return self.bench_type in NODE_TYPES

    @property
    def is_struct(self):
        """Whether this is a built-in struct type."""
        return self.bench_type in STRUCT_TYPES

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


@node(NodeType.FIELD)
class Field(Node, TypeInfoBase, _TypeExpressionBase):
    """A used-defined attribute of some value."""

    parent: Union["Block", None] = p_node_parent(4, NodeType.BLOCK)
    name: str | None = p_regular(30, default=None, validate=validate_name)
    order_key: str | None = p_internal(31, default=None)
    dynamic_key: str | None = p_internal(32, default=None)
    text: Optional["Text"] = p_regular(
        33, default=None, require=False, array=False, struct=StructType.TEXT
    )
    value_packed: Any | None = p_value_packed(34)
    value = p_value_runtime(34)

    # type identity
    # ...TypeInfo
    # ...literal_value? (for options)

    # field-only flags
    is_input: bool = p_regular(60, default=False)
    is_output: bool = p_regular(61, default=False)
    is_option: bool = p_internal(62, default=False)  # a 'literal' option (for Choice types)
    # is_indexed: bool = ... # for record fields
    # is_unique: bool = ... # for record fields (only?)

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

        flags = tuple(
            f
            for f in ("is_array", "is_required", "is_secret", "is_input", "is_output", "is_option")
            if getattr(self, f)
        )
        if flags:
            info_str += f" ({', '.join(flags)})"
        return info_str

    def _as_type(self) -> "TypeInfo":
        return self._resolved_type

    def _init_inner(self):
        if self._is_new:
            self.dynamic_key = self.dynamic_key or new_dynamic_node_key(self.ck)

    def __eq__(self, other):
        return _TypeExpressionBase.__eq__(self, other)  # override to avoid recursion

    @property
    def identifier_type(self):
        if self.is_option:
            return IdentifierType.CONSTANT
        else:
            return IdentifierType.PROPERTY

    @property
    def storage_key(self) -> str:
        return f"{self.dynamic_key}-{self.resolved_type.identity_key}"
