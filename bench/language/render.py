import base64
from dataclasses import dataclass
from datetime import datetime, timedelta
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
    node_types: Collection[NodeType] = NODE_TYPES_SET
    folded_child_types: Collection[NodeType] = (
        NodeType.FIELD,
        NodeType.TRIGGER,
        NodeType.VIEW,
        NodeType.STEP,
        NodeType.QUERY,
    )
    node_filter: Collection[UUID] | None = None


class Renderer:
    def __init__(self, options: RenderOptions):
        self._options = options

    @property
    def scope(self) -> Node:
        return self._options.scope

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
            path = find_path(scope=self.scope, node=value)
            path_str = render_path(path)
            return f"get_node({path_str!r})"
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
                return f"[{', '.join(self._render_value_object_scalar_expr(cast(ValueObject, v), typ) for v in value)}]"
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
        return ", ".join(args)

    def _remap_builtin_object_kwargs(
        self, obj: BuiltinObject, kwargs: dict[str, Any]
    ) -> dict[str, Any]:
        """Remaps the kwargs for a BuiltinObject."""
        if isinstance(obj, Field):
            # remap back to type in if possible
            type_in = reverse_type_scalar(obj)
            if type_in is not None:
                kwargs = {"type": type_in, **kwargs}
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
                    self._render_kwargs(**rendered_kwargs),
                )
            else:
                field_args = self._render_args(
                    rendered_kwargs.pop("name"), self._render_kwargs(**rendered_kwargs)
                )
            return f"Field.{constructor_name}({field_args})"
        else:
            return f"{obj.__class__.__name__}({self._render_kwargs(**rendered_kwargs)})"

    def _render_builtin_object_expr(self, obj: BuiltinObject) -> str:
        """
        Renders the given object into an expression (incl. descendants for node).
        """

        # prepare kwargs
        kwargs = _get_content_values(obj, include_defaults=False)
        kwargs = self._remap_builtin_object_kwargs(obj, kwargs)
        rendered_kwargs: dict[str, str] = {}
        for name, prop_value in kwargs.items():
            prop = obj.__properties__[name]
            rendered_kwargs[name] = self._render_value_expr(prop_value, prop.type_info)

        # fold in node children
        if isinstance(obj, Node):
            for prop in obj.__node_child_properties__.values():
                assert prop.reference_nodes, f"no reference nodes for {prop!r}"
                if prop.reference_nodes[0] not in self._options.folded_child_types:
                    continue
                children = getattr(obj, prop.name)
                if not children:
                    continue
                rendered_children = [
                    self._render_builtin_object_expr(cast(BuiltinObject, child))
                    for child in children
                ]
                rendered_kwargs[prop.name] = f"[{', '.join(rendered_children)}]"

        return self._render_builtin_object_constructor_expr(obj, kwargs, rendered_kwargs)

    def render(self, *objs: BuiltinObject | ValueObject | None) -> str:
        """Renders the given objects to Bench python and prettifies it."""
        rendered_objs: list[str] = []
        for obj in objs:
            if obj is None:
                continue  # convenience
            if isinstance(obj, ValueObject):
                rendered = self._render_value_object_scalar_expr(obj, obj._type)
            elif isinstance(obj, BuiltinObject):
                rendered = self._render_builtin_object_expr(obj)
            else:
                assert_never(obj)
            rendered_objs.append(rendered)
        rendered = "\n\n".join(rendered_objs)
        rendered = format_code(rendered)
        return rendered


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


@tracer.start_as_current_span("renderer.render")
def render(*objs: BuiltinObject | ValueObject | None, options: RenderOptions) -> str:
    """Renders the given object to Bench python and prettifies it."""
    renderer = Renderer(options)
    return renderer.render(*objs)
