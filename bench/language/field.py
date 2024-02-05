import typing
from copy import deepcopy
from typing import Any, Optional, Union

import structlog

from bench.language.const import (
    NODE_TYPES,
    STRUCT_TYPES,
    BenchType,
    BlockType,
    FormatHint,
    NodeType,
    StructType,
    new_dynamic_node_key,
)
from bench.language.node import (
    Node,
    NodeList,
    NRel,
    Property,
    ScopeNode,
    Struct,
    _TypeExpressionBase,
    node,
    p_child,
    node_component,
    p_parent,
    struct,
    p_internal,
    p_regular,
    p_runtime,
)
from bench.language.validation import validate_name
from bench.language.value import HasValue
from bench.sql.core import PrimitiveType
from bench.utils.casing import IdentifierType
from bench.utils.proxy import ProxyDict, ProxyList, unproxy_value

if typing.TYPE_CHECKING:
    from bench.language import Block, Expression
    from bench.language.notice import NoticeHandler

logger = structlog.get_logger(__name__)


class TypeError(TypeError):
    def __init__(
        self,
        value: Any,
        expected: "TypeInfo",
        message: str = None,
        suberrors: list["TypeError"] = None,
    ):
        if isinstance(value, (ProxyDict, ProxyList)):
            value = unproxy_value(value)
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


@struct(StructType.TYPE_INFO)
class TypeInfo(Struct):
    """
    A type is a kind of value that can go somewhere, typically a field.

    A type is either:
       1. primitive type (= column type, value is scalar, like int32, string, bool, datetime, ...)
       2. struct type (value is 'robust json', like Expression, File, BenchPath, RichText, ...)
       3. node type (value is NodeReference, like Package, Block, Field, Record, Run, Signal, ...)
       4. reference to a block (value is NodeReference that is an 'instance' of the block)
           node type is Record and reference ~ Database, values must be Records in that database
           node type is Run and reference ~ Block, values must be Runs of that block
           node type is Field and reference ~ Block, values must be a Field in that block
           node type is Signal and reference ~ Block, values must be Signals of that block type
           node type is Block and reference ~ Block, values must be Blocks 'implementing' that block
            (as in structural subtyping, not necessarily like Rust traits, more like Python protocols)
           node type is Block and reference is None, values must be instances of the combined newtype
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
    # visibility: NodeVisibility = struct_property(43, default=NodeVisibility.PUBLIC)
    format_hint: Optional[FormatHint] = p_regular(44, default=None)
    condition: Optional["Expression"] = p_regular(
        45, require=False, array=False, default=None, struct=StructType.EXPRESSION
    )
    length: Optional[int] = p_regular(46, require=False, default=None)
    precision: Optional[int] = p_regular(47, require=False, default=None)
    scale: Optional[int] = p_regular(48, require=False, default=None)
    # default for this type :GeneralizeHasValue
    # default: Optional[Any] = struct_property(
    #     49, require=False, default=None, primitive_type=PrimitiveType.JSON
    # )

    # flags
    is_array: bool = p_regular(50, default=False)
    is_required: bool = p_regular(51, default=False)
    is_secret: bool = p_regular(52, default=False)

    # separate _fields for restricting base type to a subset of fields (e.g., only inputs)
    _fields: tuple["Field", ...] | None = p_runtime(default=None)
    _resolved_type: Optional["TypeInfo"] = p_runtime(default=None)

    def __content_str__(self) -> str:
        if self.base_type is not None:
            info_str = self.base_type.path
        elif self.bench_type is not None:
            info_str = self.bench_type.bench_name
        elif self.primitive_type is not None:
            info_str = self.primitive_type.name
        else:
            raise ValueError(f"no type identity in {self!r}")
        if self.format_hint:
            info_str += f" as {self.format_hint}"
        if self.condition:
            info_str += f" [{self.condition}]"

        flags = tuple(
            f
            for f in ("is_array", "is_optional", "is_output", "is_secret", "is_literal")
            if getattr(self, f)
        )
        if flags:
            info_str += f" ({', '.join(flags)})"
        return info_str

    def _interp_inner(self, scope: "ScopeNode", on_notice: "NoticeHandler"):
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
        if self._fields is not None:
            return True
        elif self.base_type and HasFields in self.base_type._components:
            return True
        else:
            return False

    @property
    def fields(self) -> NodeList["Field"] | tuple["Field", ...] | None:
        if self._fields is not None:
            return self._fields
        elif self.base_type and HasFields in self.base_type._components:
            return self.base_type.fields
        else:
            return None


@node(NodeType.FIELD)
class Field(HasValue, TypeInfo, _TypeExpressionBase):
    """A used-defined attribute of some value."""

    parent: Union["Block", None] = p_parent(4, NodeType.BLOCK)
    name: str | None = p_regular(30, default=None, validate=validate_name)
    order_key: str | None = p_internal(31, default=None)
    dynamic_key: str | None = p_internal(32, default=None)
    text: str | None = p_regular(33, default=None)
    value_packed: Any | None = p_internal(
        34, default=None, copy=deepcopy, primitive_type=PrimitiveType.JSON
    )

    # type identity
    # ...TypeInfo

    # field-only flags
    is_input: bool = p_internal(60, default=False)
    is_output: bool = p_internal(61, default=False)
    is_option: bool = p_internal(62, default=False)  # a 'literal' option (for Choice types)
    # is_indexed: bool = struct_internal(63, default=False)
    # is_unique: bool = struct_internal(64, default=False)

    _introspected_from: Optional[Property] = p_runtime(default=None)

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


@node_component
class HasFields(Node):
    """A node with fields"""

    fields: NodeList["Field"] = p_child(NodeType.FIELD, NRel.NAMED | NRel.SCOPED | NRel.ORDERED)

    _did_resolve_bases: bool = p_runtime(default=False)
    _as_type: TypeInfo | None = p_runtime(default=None)

    @property
    def _type(self):
        return self._as_type

    def _init_inner(self):
        if self._is_new and self.dynamic_key is None:
            self.dynamic_key = new_dynamic_node_key(self.ck)

    def _clear_inner(self, scope: Optional[ScopeNode] = None) -> None:
        self._did_resolve_bases = False
        self._as_type = None

    def _interp_inner(self, scope: ScopeNode, on_notice: "NoticeHandler") -> None:
        self._resolve_fields([], on_notice)
        self._as_type = TypeInfo(base_type=self)

    def _resolve_fields(self: "HasFields", path: list["Node"], on_notice: "NoticeHandler") -> None:
        """
        Resolves (and inlines) field references and unions.
        """
        from bench.language import NoticeType

        if self._did_resolve_bases:
            return  # already resolved

        if any(f.id == self.id for f in path):
            # circular panic
            path = "->".join(n.name for n in path + [self])
            on_notice(type=NoticeType.CIRCULAR_BASE, subject=self, path=path)
            self._did_resolve_bases = True
            return

        # resolve fields recursively (inlining any valid unions)
        # nocheckin: Field._resolve_fields (-> resolve bases?)
        self._did_resolve_bases = True

    def _inputs_from_args(self, args, kwargs) -> dict:
        inputs = {**kwargs}
        input_fields = [f for f in self.fields if not f.is_output]
        for input_t, input in zip(input_fields, args):
            inputs[input_t.py_ident] = input
        return inputs


class TypedDict(dict):
    """
    A dot dict based on a type.
    Errors on attribute access if the field doesn't exist, otherwise returns the value (or None).
    """

    _PROPS = ("_type", "_is_output")

    def __init__(self, d: dict, type: "TypeInfo", is_output: bool = None):
        super().__init__(**d)
        self._type = type
        self._is_output = is_output

    def __str__(self):
        return super().__str__()

    def __repr__(self):
        kwargs_str = ", ".join(f"{k}={v!r}" for k, v in self.items())
        return f"{self._type.base_type.py_ident}({kwargs_str})"

    def __getitem__(self, item):
        try:
            return dict.__getitem__(self, item)
        except KeyError:
            field = self._type.fields.get(item)
            if field and (self._is_output is None or field.is_output == self._is_output):
                return None
        raise KeyError(f"no key {item!r} on {self._type!r}")

    def __getattr__(self, item):
        if item in TypedDict._PROPS:
            return super().__getattr__(item)
        try:
            return dict.__getitem__(self, item)
        except KeyError:
            field = self._type.fields.get(item)
            if field and (self._is_output is None or field.is_output == self._is_output):
                return None
        raise AttributeError(f"no attribute {item!r} on {self._type!r}")

    def __setattr__(self, name, value):
        if name in TypedDict._PROPS:
            return super().__setattr__(name, value)

        field = self._type.fields.get(name)
        if field and (self._is_output is None or field.is_output == self._is_output):
            return dict.__setitem__(self, name, value)
        raise AttributeError(f"cannot set attribute {name!r} on {self._type!r}")

    def to_dict(self):  # :ToDict
        return self
