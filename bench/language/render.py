import base64
import re
from dataclasses import dataclass
from datetime import datetime, timedelta
from enum import Enum
from typing import TYPE_CHECKING, Any, Collection, assert_never, cast, override
from uuid import UUID

import structlog
from opentelemetry import trace

from bench.language.block import Block
from bench.language.code import Code, format_code
from bench.language.const import (
    NODE_TYPES_SET,
    EnumType,
    FieldZone,
    NodeType,
    ObjectType,
    PrimitiveType,
    StructType,
    TypeKind,
)
from bench.language.field import Field, TypeConstraint, TypeInfoBase, reverse_type_scalar
from bench.language.node import BuiltinObject, Node, Struct
from bench.language.path import get_path, render_path
from bench.language.property import Property
from bench.language.setup import ENUM_CLASS_BY_TYPE, NODE_CLASS_BY_TYPE
from bench.language.text import Text
from bench.language.value import ScalarValue, SomeValue, ValueObject
from bench.language.view import View

if TYPE_CHECKING:
    pass

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


@dataclass(slots=True)
class RenderOptions:
    scope: Node
    format: bool = True
    format_line_length: int = 100
    node_types: Collection[NodeType] = NODE_TYPES_SET
    folded_child_types: Collection[NodeType] = (NodeType.FIELD, NodeType.TRIGGER, NodeType.QUERY)
    node_filter: Collection[UUID] | None = None
    stmt_separator: str = "\n"


class BuiltinObjectRenderer[T: BuiltinObject]:
    """A custom renderer for a specific builtin object."""

    def render(self, renderer: "Renderer", obj: T) -> str:
        """Render the given object to a Python expression."""
        # prepare kwargs
        kwargs = _get_content_values(obj, include_defaults=False)
        kwargs = self.map_kwargs(renderer, obj, kwargs)
        rendered_kwargs: dict[str, str] = {}
        for name, value in kwargs.items():
            if name in obj.__properties__:
                prop = obj.__properties__[name]
                rendered_kwargs[name] = renderer.render_value_expr(value, prop.type_info)
            else:
                # can pass extra kwargs that aren't real properties
                assert type(value) is str, f"unexpected kwarg str {name}={value!r}"
                rendered_kwargs[name] = value

        # fold in node children
        if isinstance(obj, Node):
            for prop in obj.__node_child_properties__.values():
                assert prop.reference_nodes, f"no reference nodes for {prop!r}"
                if prop.reference_nodes[0] not in renderer._options.folded_child_types:
                    continue
                children = getattr(obj, prop.name)
                if not children:
                    continue
                if renderer._options.node_filter is not None:
                    children = [
                        child for child in children if child.id in renderer._options.node_filter
                    ]
                rendered_children = [
                    renderer.render_builtin_object_expr(cast(BuiltinObject, child))
                    for child in children
                ]
                rendered_kwargs[prop.name] = f"[{', '.join(rendered_children)}]"

        return self.render_constructor(renderer, obj, kwargs, rendered_kwargs)

    def map_kwargs(self, renderer: "Renderer", obj: T, kwargs: dict[str, Any]) -> dict[str, Any]:
        """Remaps the kwargs for a BuiltinObject."""
        return kwargs

    def render_constructor(
        self,
        renderer: "Renderer",
        obj: T,
        kwargs: dict[str, Any],
        rendered_kwargs: dict[str, str],
    ) -> str:
        """Create the constructor expression for a BuiltinObject."""
        return f"{obj.__class__.__name__}({renderer._render_kwargs(**rendered_kwargs)})"


DEFAULT_BUILTIN_OBJECT_RENDERER = BuiltinObjectRenderer[BuiltinObject]()
_renderers: dict[ObjectType, BuiltinObjectRenderer] = {}


def _renderer(object_type: ObjectType):
    """Decorator to register a Rewriter for a specific ObjectType."""

    def decorator(cls):
        if object_type in _renderers:
            raise RuntimeError(f"rewriter for {object_type!r} already registered")
        _renderers[object_type] = cls()

    return decorator


def _get_renderer(object_type: ObjectType) -> BuiltinObjectRenderer:
    return _renderers.get(object_type, DEFAULT_BUILTIN_OBJECT_RENDERER)


class Renderer:
    """A renderer for one pass of rendering related objects (in one scope)."""

    def __init__(self, options: RenderOptions):
        self._options = options
        self._alias_by_node_id: dict[UUID, str] = {}
        self._node_by_alias: dict[str, Node] = {}

    def __str__(self) -> str:
        return f"scope={self.scope!r}, aliases={', '.join(self._node_by_alias)}"

    def __repr__(self) -> str:
        return f"<Renderer {self}>"

    @property
    def scope(self) -> Node:
        return self._options.scope

    def add_node(self, obj: Node) -> str:
        """Adds the given nodes to the context of this renderer."""
        if obj.id in self._alias_by_node_id:
            return self._alias_by_node_id[obj.id]  # already assigned
        alias = getattr(obj, "name") if hasattr(obj, "name") else obj.metatype.bench_name.lower()
        # ensure alias is valid python identifier
        if not alias:
            alias = obj.metatype.bench_name.lower()
        elif not re.match(r"^[a-zA-Z_]\w*$", alias):
            alias = f"{obj.metatype.bench_name.lower()}_{alias}"
        # bump digit at end if already exists
        if alias in self._node_by_alias:
            count = re.search(r"\d+$", alias)
            if count:
                count = int(count.group())
                alias = re.sub(r"\d+$", str(count + 1), alias)
            else:
                alias += "2"
        self._alias_by_node_id[obj.id] = alias
        self._node_by_alias[alias] = obj
        return alias

    def render_node_ref(self, node: Node) -> str:
        """Renders a python-valid reference to the given node in this context."""
        alias = self._alias_by_node_id.get(node.id)
        if alias is not None:
            return alias
        elif node._is_attached:
            path = get_path(scope=self.scope, node=node)
            path_str = render_path(path)
            return f"get_node({path_str!r})"
        else:
            raise RuntimeError(f"node {node!r} is not attached and has no alias")

    def render_value_scalar_expr(self, value: "ScalarValue", typ: "TypeInfoBase") -> str:
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
            return self.render_node_ref(value)
        elif typ.kind == TypeKind.ENUM:
            enum_cls = ENUM_CLASS_BY_TYPE[cast(EnumType, typ.bench_type)]
            value = enum_cls(value)
            return f"{enum_cls.__name__}.{value.name}"
        elif typ.kind == TypeKind.STRUCT:
            if isinstance(value, Property):
                return f"{value.component.__name__}.get_property({value.name!r})"
            else:
                return self.render_builtin_object_expr(cast(Struct, value))
        else:
            raise RuntimeError(f"unexpected type {typ!r}")

    def render_value_object_scalar_expr(self, value: "ValueObject", typ: "TypeInfoBase") -> str:
        """Renders single Object into an expression."""
        assert typ.base_type is not None, f"{value!r} has no base type"
        repr_by_name: dict[str, str] = {}
        for field in typ._base_fields:
            if typ.base_field_zone is not None and field.zone != typ.base_field_zone:
                continue
            field_type = field._to_resolved()
            field_value = cast(SomeValue, getattr(value, field.name, None))
            field_value_repr = self.render_value_expr(field_value, field_type)
            repr_by_name[field.name] = field_value_repr
        return f"{typ.base_type.name}({', '.join(f'{k}={v}' for k, v in repr_by_name.items())})"

    def render_value_expr(self, value: "SomeValue | None", typ: "TypeInfoBase") -> str:
        """Renders a value into an expression."""
        from bench.language.value import ValueObject

        if value is None:
            return "None"
        typ = typ._to_resolved()
        assert typ.kind != TypeKind.ALIAS, f"unresolved type {typ!r}"
        if typ.kind == TypeKind.OBJECT:
            # nested object
            if not typ.is_list:
                return self.render_value_object_scalar_expr(cast(ValueObject, value), typ)
            else:
                return f"[{', '.join(self.render_value_object_scalar_expr(cast(ValueObject, v), typ) for v in cast(list, value))}]"
        else:
            # scalar
            if not typ.is_list:
                return self.render_value_scalar_expr(cast("ScalarValue", value), typ)
            else:
                return f"[{', '.join(self.render_value_scalar_expr(v, typ) for v in cast(list, value))}]"

    def _render_kwargs(self, **kwargs: Any) -> str:
        """Renders kwargs into a string."""
        return ", ".join(f"{k}={v}" for k, v in kwargs.items())

    def _render_args(self, *args: Any) -> str:
        """Renders args into a string."""
        return ", ".join(a for a in args if a is not None)

    def render_builtin_object_expr(self, obj: BuiltinObject) -> str:
        """
        Renders the given object into an expression (incl. some descendants for node).
        """
        renderer = _get_renderer(obj.metatype)
        return renderer.render(self, obj)

    @tracer.start_as_current_span("renderer.render_object_expr")
    def render_obj_expr(self, obj: BuiltinObject | ValueObject):
        """Renders the given objects to a Python expression."""
        if isinstance(obj, ValueObject):
            return self.render_value_object_scalar_expr(obj, obj._type)
        elif isinstance(obj, BuiltinObject):
            return self.render_builtin_object_expr(obj)
        else:
            assert_never(obj)

    @tracer.start_as_current_span("renderer.render_stmt")
    def render_stmt(self, *objs: Node) -> str:
        """Renders the given objects to a Python block where the objects are defined."""
        # render
        rendered_objs = []
        for obj in objs:
            rendered = self.render_obj_expr(obj)
            obj_ref = self._alias_by_node_id[obj.id]
            rendered_objs.append(f"{obj_ref} = {rendered}")
            if obj.parent_ptr and obj.parent_ptr.id in self._alias_by_node_id:
                # append to parent
                parent_ref = self._alias_by_node_id[cast(UUID, obj.parent_ptr.id)]
                parent_cls = NODE_CLASS_BY_TYPE[obj.parent_ptr.type]
                parent_child_prop = parent_cls.get_node_child_property(obj.metatype)
                rendered_objs.append(f"{parent_ref}.{parent_child_prop.name}.append({obj_ref})")
        rendered = self._options.stmt_separator.join(rendered_objs)
        return rendered


def _get_content_values(obj: BuiltinObject, *, include_defaults: bool = False) -> dict[str, Any]:
    """Gets the 'content' values for a BuiltinObject."""
    values: dict[str, Any] = {}
    for prop in obj.__properties__.values():
        if prop.id is None or prop.id < 30 or prop.reference_source or prop.name == "order_key":
            continue
        value = getattr(obj, prop.name)
        if (
            (value is None and prop.default is None)
            or (isinstance(value, Collection) and len(value) == 0)
            or (value is prop.default and not include_defaults)
        ):
            continue
        values[prop.name] = value
    return values


@tracer.start_as_current_span("render.render_value_expr")
def render_value_expr(value: SomeValue, typ: TypeInfoBase, options: RenderOptions) -> str:
    """Render the given value to a python expression."""
    renderer = Renderer(options)
    rendered = renderer.render_value_expr(value, typ)
    return rendered


@tracer.start_as_current_span("render.render_expr")
def render_expr(
    value: BuiltinObject | ValueObject, options: RenderOptions, as_ref: bool = False
) -> str:
    """Render the given object to a python expression."""
    renderer = Renderer(options)
    rendered: str
    if isinstance(value, ValueObject):
        rendered = renderer.render_value_object_scalar_expr(value, value._type)
    elif isinstance(value, BuiltinObject):
        if isinstance(value, Node) and as_ref:
            rendered = renderer.render_node_ref(value)
        else:
            rendered = renderer.render_builtin_object_expr(value)
    else:
        assert_never(value)
    if options.format:
        rendered = format_code(rendered, line_length=options.format_line_length)
    return rendered.strip()


@tracer.start_as_current_span("render.render_stmt")
def render_stmt(*objs: Node, options: RenderOptions) -> str:
    """Renders the given object to a python block where the objects are defined."""
    renderer = Renderer(options)
    for obj in objs:
        renderer.add_node(obj)
    rendered = renderer.render_stmt(*objs)
    if options.format:
        rendered = format_code(rendered, line_length=options.format_line_length)
    return rendered.strip()


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


#
# Specific renderers
#


@_renderer(NodeType.BLOCK)
class BlockRenderer(BuiltinObjectRenderer[Block]):
    @override
    def render_constructor(
        self,
        renderer: "Renderer",
        obj: Block,
        kwargs: dict[str, Any],
        rendered_kwargs: dict[str, str],
    ) -> str:
        block_args = renderer._render_args(
            rendered_kwargs.pop("type"),
            rendered_kwargs.pop("name"),
            renderer._render_kwargs(**rendered_kwargs) or None,
        )
        return f"Block.new({block_args})"


@_renderer(NodeType.VIEW)
class ViewRenderer(BuiltinObjectRenderer[View]):
    @override
    def render_constructor(
        self,
        renderer: "Renderer",
        obj: View,
        kwargs: dict[str, Any],
        rendered_kwargs: dict[str, str],
    ) -> str:
        view_args = renderer._render_args(
            rendered_kwargs.pop("type"),
            rendered_kwargs.pop("name"),
            renderer._render_kwargs(**rendered_kwargs) or None,
        )
        return f"View.new({view_args})"


@_renderer(NodeType.FIELD)
class FieldRenderer(BuiltinObjectRenderer[Field]):
    @override
    def map_kwargs(
        self, renderer: "Renderer", obj: Field, kwargs: dict[str, Any]
    ) -> dict[str, Any]:
        # remap back to type in if possible
        type_in = reverse_type_scalar(obj)
        if type_in is not None:
            if isinstance(type_in, Node):
                rendered_type = renderer.render_node_ref(type_in)
            elif isinstance(type_in, Enum):
                rendered_type = f"{type_in.__class__.__name__}.{type_in.name}"
            else:
                assert isinstance(type_in, type), f"unexpected type {type_in!r}"
                rendered_type = type_in.__name__
            kwargs = {"type": rendered_type, **kwargs}
            for key in ("kind", "primitive_type", "bench_type", "base_type"):
                kwargs.pop(key, None)
        # kind=literal is implicit if option
        if obj.zone == FieldZone.OPTION:
            kwargs.pop("kind")
        return kwargs

    @override
    def render_constructor(
        self,
        renderer: "Renderer",
        obj: Field,
        kwargs: dict[str, Any],
        rendered_kwargs: dict[str, str],
    ) -> str:
        constructor_name = obj.zone.name.lower()
        rendered_kwargs.pop("zone", None)
        if "type" in rendered_kwargs:
            field_args = renderer._render_args(
                rendered_kwargs.pop("name"),
                rendered_kwargs.pop("type"),
                renderer._render_kwargs(**rendered_kwargs) or None,
            )
        else:
            field_args = renderer._render_args(
                rendered_kwargs.pop("name"), renderer._render_kwargs(**rendered_kwargs) or None
            )
        return f"Field.{constructor_name}({field_args})"


@_renderer(StructType.TYPE_CONSTRAINT)
class TypeConstraintRenderer(BuiltinObjectRenderer[TypeConstraint]):
    @override
    def render_constructor(
        self,
        renderer: "Renderer",
        obj: TypeConstraint,
        kwargs: dict[str, Any],
        rendered_kwargs: dict[str, str],
    ) -> str:
        return f"constrain({renderer._render_kwargs(**rendered_kwargs)})"


@_renderer(StructType.TEXT)
class TextRenderer(BuiltinObjectRenderer[Text]):
    @override
    def render(self, renderer: "Renderer", obj: Text) -> str:
        return f"md({obj.to_markdown()!r})"


@_renderer(StructType.CODE)
class CodeRenderer(BuiltinObjectRenderer[Code]):
    @override
    def render(self, renderer: "Renderer", obj: Code) -> str:
        return f"code({obj.to_string()!r})"
