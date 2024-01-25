import typing
from copy import deepcopy
from typing import Any, Optional, Union

import structlog

from bench.language.const import (
    IssueType,
    NodeType,
    new_dynamic_node_key,
    BenchType,
    StructType,
    FormatHint,
    BlockType,
)
from bench.language.node import (
    Node,
    NodeList,
    NRel,
    Property,
    ScopeNode,
    _FieldExpressionBase,
    node,
    node_children,
    node_component,
    node_parent,
    struct_internal,
    struct_property,
    struct_runtime,
    Struct,
    struct,
)
from bench.language.validation import (
    validate_is_str,
    validate_name,
)
from bench.language.value import HasValue
from bench.sql.core import ColumnType
from bench.utils.casing import IdentifierType
from bench.utils.proxy import ProxyDict, ProxyList, unproxy_value

if typing.TYPE_CHECKING:
    from bench.language import Block, Expression
    from bench.language.issue import IssueHandler

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
       1. built-in type (= column type, value is scalar, like int32, string, bool, datetime, ...)
       2. struct type (value is 'robust json', like Expression, File, BenchPath, RichText, ...)
       3. node type (value is NodeReference, like Package, Block, Field, Record, Run, Signal, ...)
       4. reference to a block (value is NodeReference that is an 'instance' of the block)
           if node type is Record and reference ~ Database, values must be Records in that database
           if node type is Run and reference ~ Block, values must be Runs of that block
           if node type is Field and reference ~ Block, values must be a Field in that block
           if node type is Signal and reference ~ Block, values must be Signals of that block type
           if node type is Block and reference ~ Block, values must be Blocks 'implementing' that block
           if node type is Block and reference is None, values must be instances of the combined newtype
            ...

    Types may also specify:
       - a format hint (which may impact the unpacked/instantiated Python representation, like for Image)
       - an additional condition instances must satisfy
       - combination flags for arrays, optionals, ...

    Type checking is done in ./value.py. You'll note that we can only check some things without querying.
    """

    # type identity (must set at least one of these)
    column_type: Optional[ColumnType] = struct_property(40, default=None)
    bench_type: Optional[BenchType] = struct_property(41, default=None)
    base_type: Optional["Block"] = struct_property(
        42, array=False, require=False, default=None, references=NodeType.BLOCK
    )
    # + bonus info/constraints
    format_hint: Optional[FormatHint] = struct_property(43, default=None)
    condition: Optional["Expression"] = struct_property(
        44, require=False, array=False, default=None, struct=StructType.EXPRESSION
    )
    # visibility: NodeVisibility = ...?

    # flags
    is_array: bool = struct_internal(50, default=False)
    is_optional: bool = struct_internal(51, default=True)
    is_output: bool = struct_internal(52, default=False)
    is_secret: bool = struct_internal(53, default=False)
    is_literal: bool = struct_internal(54, default=False)

    # separate _fields for restricting base type to a subset of fields (e.g., only inputs)
    _fields: tuple["Field", ...] | None = struct_runtime(default=None)
    _derived_type: Optional["TypeInfo"] = struct_runtime(default=None)

    def __content_str__(self) -> str:
        if self.base_type is not None:
            info_str = self.base_type.path
        elif self.bench_type is not None:
            info_str = self.bench_type.bench_name
        elif self.column_type is not None:
            info_str = self.column_type.name
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

    def _interp_inner(self, scope: "ScopeNode", on_issue: "IssueHandler"):
        # NOTE: TypeInfo.interp / derivation probably isn't quite right yet
        if self.base_type is not None:
            if self.base_type_type == BlockType.ALIAS:  # newtype
                raise NotImplementedError("newtypes are not supported yet")
            elif self.base_type_type == BlockType.CHOICE and self.bench_type == NodeType.FIELD:
                self._derived_type = self.extend(column_type=ColumnType.STRING)
            else:
                self._derived_type = self.extend(column_type=ColumnType.UUID)
        else:
            self._derived_type = self

    def extend(self, **kwargs) -> "TypeInfo":
        """Returns a new type that is the same as this one, but with the given properties overridden."""
        combined_kwargs = {
            prop.name: getattr(self, prop.name) for prop in TypeInfo.__runtime_properties__.values()
        }
        for k, v in kwargs.items():
            combined_kwargs[k] = v
        return TypeInfo(**combined_kwargs)

    @property
    def derived_type(self) -> "TypeInfo":
        assert self._derived_type is not None, f"derived type not ready in {self!r}"
        return self._derived_type

    @property
    def derived_column_type(self) -> ColumnType:
        return self.column_type or self.derived_type.column_type

    @property
    def identity_key(self) -> str:
        """The identity of this type for storing. Different keys mean you won't get the value back out."""
        assert self.column_type is not None, f"no column type in {self!r}"
        key = str(self.column_type.id)
        if self.is_array:
            key += "a"
        if self.is_secret:
            key += "s"
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
    def fields(self) -> NodeList["Field"] | tuple["Field", ...] | None:
        if self._fields is not None:
            return self._fields
        elif self.base_type and HasFields in self.base_type._components:
            return self.base_type.fields
        else:
            return None


@node(NodeType.FIELD)
class Field(HasValue, TypeInfo, _FieldExpressionBase):
    parent: Union["Block", None] = node_parent(4, NodeType.BLOCK)
    name: str | None = struct_property(30, default=None, validate=validate_name)
    order_key: str | None = struct_internal(31, default=None)
    dynamic_key: str | None = struct_internal(32, default=None)
    text: str | None = struct_property(33, default=None, validate=validate_is_str)
    value_packed: Any | None = struct_property(
        34, default=None, copy=deepcopy, column_type=ColumnType.JSON
    )

    # type identity
    # ...TypeInfo

    _reflected_from: Optional[Property] = struct_runtime(default=None)

    def _init_inner(self):
        if self._is_new:
            self.dynamic_key = self.dynamic_key or new_dynamic_node_key(self.ck)

    def __eq__(self, other):
        return _FieldExpressionBase.__eq__(self, other)  # override to avoid recursion

    @property
    def identifier_type(self):
        if self.is_literal:
            return IdentifierType.CONSTANT
        else:
            return IdentifierType.PROPERTY

    @property
    def storage_key(self) -> str:
        return f"{self.dynamic_key}-{self.derived_type.identity_key}"


@node_component
class HasFields(Node):
    """A node with fields"""

    fields: NodeList["Field"] = node_children(
        NodeType.FIELD, NRel.NAMED | NRel.SCOPED | NRel.ORDERED
    )

    _did_resolve_bases: bool = struct_runtime(default=False)
    _as_type_info: TypeInfo | None = struct_runtime(default=None)

    @property
    def _type(self):
        return self._as_type_info

    def _init_inner(self):
        if self._is_new and self.dynamic_key is None:
            self.dynamic_key = new_dynamic_node_key(self.ck)

    def _clear_inner(self, scope: Optional[ScopeNode] = None) -> None:
        self._did_resolve_bases = False
        self._as_type_info = None

    def _interp_inner(self, scope: ScopeNode, on_issue: "IssueHandler") -> None:
        self._resolve_fields([], on_issue)
        self._as_type_info = TypeInfo(base_type=self)

    def _resolve_fields(self: "HasFields", path: list["Node"], on_issue: "IssueHandler") -> None:
        """
        Resolves (and inlines) field references and unions.
        """
        if self._did_resolve_bases:
            return  # already resolved

        if any(f.id == self.id for f in path):
            # circular panic
            path = "->".join(n.name for n in path + [self])
            on_issue(type=IssueType.CIRCULAR_BASE, subject=self, path=path)
            self._did_resolve_bases = True
            return

        # resolve fields recursively (inlining any valid unions)
        # nocheckin: Field._resolve_fields -> resolve bases
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
