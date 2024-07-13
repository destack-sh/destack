import base64
import re
from dataclasses import dataclass
from datetime import datetime, timedelta
from enum import Enum
from typing import TYPE_CHECKING, Any, Collection, assert_never, cast
from uuid import UUID

import structlog
from opentelemetry import trace

from bench.language.code import format_code
from bench.language.const import (
    NODE_TYPES_SET,
    EnumType,
    FieldZone,
    NodeType,
    PrimitiveType,
    TypeKind,
)
from bench.language.field import Field, reverse_type_scalar
from bench.language.node import BuiltinObject, Node, Struct
from bench.language.path import find_path, render_path
from bench.language.setup import ENUM_CLASS_BY_TYPE
from bench.language.value import ScalarValue, SomeValue, ValueObject

if TYPE_CHECKING:
    from bench.language.field import TypeInfoBase

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


@dataclass(slots=True)
class RenderOptions:
    scope: Node
    format: bool = True
    node_types: Collection[NodeType] = NODE_TYPES_SET
    folded_child_types: Collection[NodeType] = (
        NodeType.FIELD,
        NodeType.TRIGGER,
        NodeType.QUERY,
    )
    node_filter: Collection[UUID] | None = None
    block_separator: str = "\n"


class Renderer:
    """A renderer for one pass of rendering related objects (at the same scope)."""

    def __init__(self, options: RenderOptions):
        self._options = options
        self._alias_by_node_id: dict[UUID, str] = {}
        self._node_by_alias: dict[str, Node] = {}

    @property
    def scope(self) -> Node:
        return self._options.scope

    def _render_node_ref(self, node: Node) -> str:
        if node._is_attached:
            path = find_path(scope=self.scope, node=node)
            path_str = render_path(path)
            return f"get_node({path_str!r})"
        else:
            alias = self._alias_by_node_id.get(node.id)
            assert alias is not None, f"no alias for {node!r}"
            return alias

    def _render_value_scalar_expr(self, value: "ScalarValue", typ: "TypeInfoBase") -> str:
        """Renders single scalar value into an expression."""
        if typ.kind == TypeKind.PRIMITIVE:
            if typ.primitive_type == PrimitiveType.BYTES:
                value_b64 = base64.b64encode(cast(bytes, value)).decode("utf-8")
                return f"base64.b64decode({value_b64!r})"
            elif typ.primitive_type == PrimitiveType.DATETIME:
                value_iso = cast(datetime, value).isoformat()
                return f"datetime.fromisoformat({value_iso!r})"
            elif typ.primitive_type == PrimitiveType.INTERVAL:
                return f"timedelta(seconds={cast(timedelta, value).total_seconds()})"
            else:
                return repr(value)
        elif typ.kind == TypeKind.NODE or typ.kind == TypeKind.BASED_NODE:
            assert isinstance(value, Node), f"{value!r} is not a node (expected {typ!r})"
            return self._render_node_ref(value)
        elif typ.kind == TypeKind.ENUM:
            enum_cls = ENUM_CLASS_BY_TYPE[cast(EnumType, typ.bench_type)]
            value = enum_cls(value)
            return f"{enum_cls.__name__}.{value.name}"
        elif typ.kind == TypeKind.STRUCT:
            return self._render_builtin_object_expr(cast(Struct, value))
        else:
            raise RuntimeError(f"unexpected type {typ!r}")

    def _render_value_object_scalar_expr(self, value: "ValueObject", typ: "TypeInfoBase") -> str:
        """Renders single Object into an expression."""
        assert typ.base_type is not None, f"{value!r} has no base type"
        repr_by_name: dict[str, str] = {}
        for field in typ._base_fields:
            field_type = field._to_resolved()
            field_value = cast(SomeValue, getattr(value, field.name, None))
            field_value_repr = self._render_value_expr(field_value, field_type)
            repr_by_name[field.name] = field_value_repr
        return f"{typ.base_type.name}({', '.join(f'{k}={v}' for k, v in repr_by_name.items())})"

    def _render_value_expr(self, value: "SomeValue | None", typ: "TypeInfoBase") -> str:
        """Renders a value into an expression."""
        from bench.language.value import ValueObject

        if value is None:
            return "None"
        typ = typ._to_resolved()
        assert typ.kind != TypeKind.ALIAS, f"unresolved type {typ!r}"
        if typ.kind == TypeKind.OBJECT:
            # nested object
            if not typ.is_list:
                return self._render_value_object_scalar_expr(cast(ValueObject, value), typ)
            else:
                return f"[{', '.join(self._render_value_object_scalar_expr(cast(ValueObject, v), typ) for v in cast(list, value))}]"
        else:
            # scalar
            if not typ.is_list:
                return self._render_value_scalar_expr(cast("ScalarValue", value), typ)
            else:
                return f"[{', '.join(self._render_value_scalar_expr(v, typ) for v in cast(list, value))}]"

    def _render_kwargs(self, **kwargs: Any) -> str:
        """Renders kwargs into a string."""
        return ", ".join(f"{k}={v}" for k, v in kwargs.items())

    def _render_args(self, *args: Any) -> str:
        """Renders args into a string."""
        return ", ".join(a for a in args if a is not None)

    def _remap_builtin_object_kwargs(
        self, obj: BuiltinObject, kwargs: dict[str, Any]
    ) -> dict[str, Any]:
        """Remaps the kwargs for a BuiltinObject."""
        if isinstance(obj, Field):
            # remap back to type in if possible
            type_in = reverse_type_scalar(obj)
            if type_in is not None:
                if isinstance(type_in, Node):
                    rendered_type = self._render_node_ref(type_in)
                elif isinstance(type_in, Enum):
                    rendered_type = f"{type_in.__class__.__name__}.{type_in.name}"
                else:
                    rendered_type = repr(type_in)
                kwargs = {"type": rendered_type, **kwargs}
                for key in ("kind", "primitive_type", "bench_type", "base_type"):
                    kwargs.pop(key, None)
            # kind=literal is implicit if option
            if obj.zone == FieldZone.OPTION:
                kwargs.pop("kind")
        return kwargs

    def _render_builtin_object_constructor_expr(
        self, obj: BuiltinObject, kwargs: dict[str, Any], rendered_kwargs: dict[str, str]
    ) -> str:
        """Remaps the rendered kwargs for a BuiltinObject."""
        if obj.metatype == NodeType.BLOCK:
            block_args = self._render_args(
                rendered_kwargs.pop("type"),
                rendered_kwargs.pop("name"),
                self._render_kwargs(**rendered_kwargs),
            )
            return f"Block.new({block_args})"
        elif isinstance(obj, Field):
            constructor_name = obj.zone.name.lower()
            rendered_kwargs.pop("zone", None)  # implicit in constructor name
            if "type" in rendered_kwargs:
                field_args = self._render_args(
                    rendered_kwargs.pop("name"),
                    rendered_kwargs.pop("type"),
                    self._render_kwargs(**rendered_kwargs) or None,
                )
            else:
                field_args = self._render_args(
                    rendered_kwargs.pop("name"), self._render_kwargs(**rendered_kwargs) or None
                )
            return f"Field.{constructor_name}({field_args})"
        else:
            return f"{obj.__class__.__name__}({self._render_kwargs(**rendered_kwargs) or None})"

    def _render_builtin_object_expr(self, obj: BuiltinObject) -> str:
        """
        Renders the given object into an expression (incl. some descendants for node).
        """

        # prepare kwargs
        kwargs = _get_content_values(obj, include_defaults=False)
        kwargs = self._remap_builtin_object_kwargs(obj, kwargs)
        rendered_kwargs: dict[str, str] = {}
        for name, value in kwargs.items():
            if name in obj.__properties__:
                prop = obj.__properties__[name]
                rendered_kwargs[name] = self._render_value_expr(value, prop.type_info)
            else:
                # can pass extra kwargs that aren't real properties
                assert type(value) is str, f"unexpected kwarg str {name}={value!r}"
                rendered_kwargs[name] = value

        # fold in node children
        if isinstance(obj, Node):
            for prop in obj.__node_child_properties__.values():
                assert prop.reference_nodes, f"no reference nodes for {prop!r}"
                if prop.reference_nodes[0] not in self._options.folded_child_types:
                    continue
                children = getattr(obj, prop.name)
                if not children:
                    continue
                if self._options.node_filter is not None:
                    children = [
                        child for child in children if child.id in self._options.node_filter
                    ]
                rendered_children = [
                    self._render_builtin_object_expr(cast(BuiltinObject, child))
                    for child in children
                ]
                rendered_kwargs[prop.name] = f"[{', '.join(rendered_children)}]"

        return self._render_builtin_object_constructor_expr(obj, kwargs, rendered_kwargs)

    def render_obj_expr(self, obj: BuiltinObject | ValueObject):
        """Renders the given objects to a Python expression."""
        if isinstance(obj, ValueObject):
            return self._render_value_object_scalar_expr(obj, obj._type)
        elif isinstance(obj, BuiltinObject):
            return self._render_builtin_object_expr(obj)
        else:
            assert_never(obj)

    def render_stmt(self, *objs: Node) -> str:
        """Renders the given objects to a Python block where the objects are defined."""
        # assign aliases
        for obj in objs:
            if obj.id in self._alias_by_node_id:
                continue  # already assigned
            if hasattr(obj, "name"):
                alias = getattr(obj, "name")
            else:
                alias = obj.metatype.bench_name.lower()
            if alias in self._node_by_alias:
                # add/increment digit at end
                count = re.search(r"\d+$", alias)
                if count:
                    count = int(count.group())
                    alias = re.sub(r"\d+$", str(count + 1), alias)
                else:
                    alias += "2"
            self._alias_by_node_id[obj.id] = alias
            self._node_by_alias[alias] = obj

        # render
        rendered_objs = []
        for obj in objs:
            rendered = self.render_obj_expr(obj)
            is_parent_in_scope = (
                obj.parent_ptr is not None and obj.parent_ptr.id in self._alias_by_node_id
            )
            rendered_objs.append(f"{self._alias_by_node_id[obj.id]} = {rendered}")
            if is_parent_in_scope:
                # append to parent
                ...  # nocheckin

        # combine
        rendered = self._options.block_separator.join(rendered_objs)
        if self._options.format:
            rendered = format_code(rendered)
        return rendered.strip()


def _get_content_values(obj: BuiltinObject, *, include_defaults: bool = False) -> dict[str, Any]:
    """Gets the 'content' values for a BuiltinObject."""
    values: dict[str, Any] = {}
    for prop in obj.__properties__.values():
        if prop.id is None or prop.id < 30 or prop.reference_source or prop.name == "order_key":
            continue
        value = getattr(obj, prop.name)
        if (
            value is None
            or (isinstance(value, Collection) and len(value) == 0)
            or (value is prop.default and not include_defaults)
        ):
            continue
        values[prop.name] = value
    return values


@tracer.start_as_current_span("renderer.render_expr")
def render_expr(obj: BuiltinObject | ValueObject, options: RenderOptions) -> str:
    """Render the given object to a python expression."""
    renderer = Renderer(options)
    rendered: str
    if isinstance(obj, ValueObject):
        rendered = renderer._render_value_object_scalar_expr(obj, obj._type)
    elif isinstance(obj, BuiltinObject):
        rendered = renderer._render_builtin_object_expr(obj)
    else:
        assert_never(obj)
    if options.format:
        rendered = format_code(rendered)
    return rendered


@tracer.start_as_current_span("renderer.render_stmt")
def render_stmt(*objs: Node, options: RenderOptions) -> str:
    """Renders the given object to a python block where the objects are defined."""
    renderer = Renderer(options)
    return renderer.render_stmt(*objs)


def render(*objs: BuiltinObject | ValueObject, options: RenderOptions) -> str:
    """Renders the given object to either an expression (for values) or statement (for nodes)."""
    if any(isinstance(obj, Node) for obj in objs):
        # render into single statement block
        assert all(
            isinstance(obj, Node) for obj in objs
        ), f"cannot render nodes with other values: {objs!r}"
        return render_stmt(*cast(list[Node], objs), options=options)
    else:
        # render into tuple of expressions
        value_exprs = [render_expr(obj, options) for obj in objs]
        return ", ".join(value_exprs)
