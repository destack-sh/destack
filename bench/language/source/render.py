import base64
import dataclasses
from dataclasses import dataclass
from datetime import date, datetime, time, timedelta
from enum import Enum
from typing import TYPE_CHECKING, Any, Collection, Mapping, assert_never, cast, overload, override
from uuid import UUID

import regex
import structlog
from opentelemetry import trace

from bench.language.core import (
    NODE_TYPES_SET,
    BuiltinObject,
    Code,
    ComputedValue,
    ComputedValueMode,
    CustomObject,
    EnumType,
    FieldType,
    Icon,
    Node,
    NodeReference,
    NodeType,
    ObjectType,
    Path,
    PathElement,
    PathElementType,
    PrimitiveType,
    Property,
    PropertyReference,
    Resource,
    ScalarValue,
    SomeValue,
    SourceNode,
    Struct,
    StructType,
    Text,
    TypeBase,
    TypeConstraint,
    TypeFormat,
    TypeKind,
    format_code,
    get_custom_object_properties,
    get_path,
    is_node_type,
    render_path,
    reverse_icon,
    reverse_path_element,
    reverse_type_scalar,
)
from bench.language.core.text import text_to_markdown
from bench.language.registry import ENUM_CLASS_BY_TYPE, NODE_CLASS_BY_TYPE
from bench.language.runtime import (
    Call,
    CallExecutionMode,
    CallFailureMode,
    CallPlan,
    CallTerminationMode,
)
from bench.utils.time import timedelta_to_isoformat

from .action import Action
from .block import Block
from .choice import Choice
from .clazz import Class
from .database import Database
from .field import Field
from .flow import Flow
from .link import Link
from .option import Option
from .page import Page
from .view import View

if TYPE_CHECKING:
    pass

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


@dataclass(slots=True)
class RenderOptions:
    scope: Node
    aliasing: "Aliasing"
    node_types: Collection[NodeType] = NODE_TYPES_SET
    inline_node_types: Collection[NodeType] = (NodeType.FIELD, NodeType.OPTION, NodeType.TRIGGER)
    use_code_paths: bool = True
    implicit_partials: bool = False
    # formatting
    statement_separator: str = "\n"
    format: bool = True
    line_length: int = 100

    def replace(self, **kwargs) -> "RenderOptions":
        return dataclasses.replace(self, **kwargs)


class Aliasing:
    """Registry of aliases for Nodes."""

    def __init__(self):
        self._alias_by_node_id: dict[UUID, str] = {}
        self._node_by_alias: dict[str, Node | NodeReference] = {}

    def __str__(self) -> str:
        return ", ".join(self._node_by_alias)

    def __repr__(self) -> str:
        return f"<Aliasing {self}>"

    def add(self, obj: Node | NodeReference) -> str:
        """Adds the given nodes to the context of this renderer."""
        from bench.runtime.code.context import CODE_GLOBALS

        if obj.id in self._alias_by_node_id:
            return self._alias_by_node_id[obj.id]  # already assigned
        if isinstance(obj, Node) and getattr(obj, "code_name"):
            # proper given name
            alias = getattr(obj, "code_name")
            if not regex.match(r"^[a-zA-Z_]\w+$", alias):  # ensure it's a valid python identifier
                alias = f"{obj.metatype.bench_name}_{alias}"
            has_given_name = True
        else:
            alias = obj.metatype.bench_name if isinstance(obj, Node) else obj.node_type.bench_name
            has_given_name = False
        if alias in self._node_by_alias or alias in CODE_GLOBALS or not has_given_name:
            # bump digit at end to make alias unique
            count = regex.search(r"\d+$", alias)
            if count is None:
                alias = f"{alias}1"
                count = 1
            else:
                count = int(count.group())
            while alias in self._node_by_alias or alias in CODE_GLOBALS:
                count += 1
                alias = regex.sub(r"\d+$", str(count + 1), alias)
        self._alias_by_node_id[cast(UUID, obj.id)] = alias
        self._node_by_alias[alias] = obj
        return alias

    def get(self, node: Node | NodeReference | UUID) -> str | None:
        """Gets the alias for the given node."""
        if isinstance(node, Node):
            return self._alias_by_node_id.get(node.id)
        elif isinstance(node, NodeReference):
            return self._alias_by_node_id.get(cast(UUID, node.id))
        elif isinstance(node, UUID):
            return self._alias_by_node_id.get(node)
        else:
            assert_never(node)

    def get_or_error(self, node: Node | NodeReference | UUID) -> str:
        """Gets the alias for the given node (error if not found)."""
        alias = self.get(node)
        if alias is None:
            raise LookupError(f"no alias for {node!r} in {self!r}")
        return alias

    def get_or_add(self, obj: Node | NodeReference) -> str:
        """Gets the alias for the given node (add if not found)."""
        alias = self.get(obj)
        if alias is None:
            alias = self.add(obj)
        return alias

    def get_name(self, node: Node | NodeReference) -> str:
        """Gets the name for the given node."""
        alias = self.get(node)
        if alias is None:
            raise LookupError(f"no alias for {node!r} in {self!r}")
        return alias

    __getitem__ = get_or_error

    def __contains__(self, node: Node | NodeReference | UUID) -> bool:
        return self.get(node) is not None

    def resolve(self, name: str) -> Node | NodeReference | None:
        """Resolves the given name to a node."""
        return self._node_by_alias.get(name)

    @staticmethod
    def new(aliases: Mapping[str, Node | NodeReference]) -> "Aliasing":
        """Create a new Aliasing registry from a mapping of aliases to nodes."""
        aliasing = Aliasing()
        for alias, node in aliases.items():
            aliasing._alias_by_node_id[cast(UUID, node.id)] = alias
            aliasing._node_by_alias[alias] = node
        return aliasing


AliasingIn = Aliasing | Mapping[str, Node | NodeReference]


class Renderer:
    """A renderer to render related objects into code(ish)."""

    def __init__(self, options: RenderOptions):
        self.options = options
        self.aliasing = options.aliasing

    def __str__(self) -> str:
        return f"scope={self.scope!r}, aliases={', '.join(self.aliasing._node_by_alias)}"

    def __repr__(self) -> str:
        return f"<Renderer {self}>"

    @property
    def scope(self) -> Node:
        return self.options.scope

    def render_node_ref(self, node: Node | NodeReference) -> str:
        """Renders a python-valid reference to the given node in this context."""
        if (
            isinstance(node, Node)
            and node.metatype in self.options.inline_node_types
            and "name" in node.__properties__
        ):
            # refer named inlined children from parent
            parent = node.parent
            if parent is not None:
                parent_alias = self.render_node_ref(parent)
                parent_cls = NODE_CLASS_BY_TYPE[parent.metatype]
                parent_child_prop = parent_cls.get_child_property_or_error(node.metatype)
                alias = f"{parent_alias}.{parent_child_prop.name}.{node.code_name}"
                return alias

        alias = self.aliasing.get(node)
        if alias is not None:
            # already have an alias
            return alias
        elif isinstance(node, Node) and node.is_attached and "name" in node.__properties__:
            # reference as path
            if self.scope.id == node.id:
                return "self"
            path = get_path(scope=self.scope, node=node)
            rendered_path = render_path(path)
            if self.options.use_code_paths:
                # simplify path for use in Code (which treats references as unique get_node)
                if len(path) == 1 and path[0].type in (
                    PathElementType.CLOSEST,
                    PathElementType.CHILD,
                ):
                    assert path[0].code_name is not None, f"no name for {path[0]!r}"
                    return path[0].code_name
                elif (
                    len(path) == 2
                    and path[0].type in (PathElementType.CLOSEST, PathElementType.CHILD)
                    and path[1].type == PathElementType.ATTRIBUTE
                ):
                    return f"{path[0].name}.{path[1].code_name}"
            return f"get_node({rendered_path!r})"
        else:
            # create new alias
            alias = self.aliasing.add(node)
            return alias

    def render_property_ref(self, prop: Property | PropertyReference) -> str:
        if isinstance(prop, PropertyReference):
            prop = prop.resolve_or_error()
        if prop.value_runtime_ptr is not None:
            prop = prop.value_runtime_ptr
        return f'{prop.component.__name__}.get_property("{prop.name}")'

    def render_value_scalar(self, value: "ScalarValue", typ: "TypeBase") -> str:
        """Renders single scalar value into an expression."""
        if typ.kind == TypeKind.PRIMITIVE:
            if typ.primitive_type == PrimitiveType.BYTES:
                value_b64 = base64.b64encode(cast(bytes, value)).decode("utf-8")
                return f"base64.b64decode({value_b64!r})"
            elif typ.primitive_type == PrimitiveType.DATETIME:
                value_iso = cast(datetime, value).isoformat()
                return f"datetime.fromisoformat({value_iso!r})"
            elif typ.primitive_type == PrimitiveType.DATE:
                value_iso = cast(date, value).isoformat()
                return f"date.fromisoformat({value_iso!r})"
            elif typ.primitive_type == PrimitiveType.TIME:
                value_iso = cast(time, value).isoformat()
                return f"time.fromisoformat({value_iso!r})"
            elif typ.primitive_type == PrimitiveType.DURATION:
                value_iso = timedelta_to_isoformat(cast(timedelta, value))
                return f"timedelta_from_isoformat({value_iso!r})"
            elif typ.primitive_type == PrimitiveType.STRING:
                return f'"{value}"'
            elif typ.primitive_type == PrimitiveType.UUID:
                return f"UUID({value!r})"
            else:
                return str(value)
        elif typ.kind == TypeKind.NODE or typ.kind == TypeKind.BASED_NODE:
            assert isinstance(
                value, (Node, NodeReference)
            ), f"{value!r} is not a node or node reference, expected {typ!r}"
            return self.render_node_ref(value)
        elif typ.kind == TypeKind.ENUM:
            enum_cls = ENUM_CLASS_BY_TYPE[cast(EnumType, typ.bench_type)]
            value = enum_cls(cast(int, value))
            return f"{enum_cls.__name__}.{value.name}"
        elif typ.kind == TypeKind.STRUCT:
            if isinstance(value, Property):
                return f"{value.component.__name__}.get_property({value.name!r})"
            else:
                return self.render_builtin_object(cast(Struct, value))
        else:
            raise RuntimeError(f"unexpected type {typ!r}")

    def render_custom_object(self, value: "CustomObject", implicit_partials: bool = False) -> str:
        """Renders single Object into an expression."""
        # collect kwargs
        typ = value._type
        kwargs = _deconstruct_custom_object(value)
        rendered_kwargs = _render_custom_object_kwargs(self, value, kwargs)
        if value._type.kind == TypeKind.PARTIAL_OBJECT and not (
            self.options.implicit_partials or implicit_partials
        ):
            node_cls = (
                NODE_CLASS_BY_TYPE[cast(NodeType, value._type.bench_type)]
                if value._type.bench_type
                else Node
            )
            if value._type.bench_type is not None:
                rendered_kwargs.pop("metatype", None)
            if value._type.constraint is not None and value._type.constraint.node_subtypes:
                rendered_kwargs.pop("type", None)
                assert node_cls.__subtype_base_property__, f"bad subtype {node_cls!r}"
                subtype_type = node_cls.__subtype_base_property__.enum_type
                assert subtype_type is not None, f"bad subtype {node_cls!r}"
                subtype_cls = ENUM_CLASS_BY_TYPE[subtype_type]
                subtype = subtype_cls(value._type.constraint.node_subtypes[0])
                args = (
                    subtype_cls.__name__ + "." + subtype.name,
                    self._render_kwargs(**rendered_kwargs) or None,
                )
                return f"{node_cls.__name__}.partial({self._render_args(*args)})"
            else:
                return f"{node_cls.__name__}.partial({self._render_kwargs(**rendered_kwargs)})"
        elif (
            typ.base_field_types
            and FieldType.MEMBER in typ.base_field_types
            and (base_type := typ.base_type) is not None
        ):
            # object representation
            kwargs_str = f"{base_type.code_name}({self._render_kwargs(**rendered_kwargs)})"
            return kwargs_str
        else:
            # default dict representation
            kwargs_str = ", ".join(f"'{k}': {v}" for k, v in rendered_kwargs.items())
            kwargs_str = f"{{{', '.join({kwargs_str})}}}"
            return kwargs_str

    def render_value(self, value: "SomeValue | None", typ: "TypeBase") -> str:
        """Renders a value into an expression."""
        from bench.language.core import CustomObject

        if value is None:
            return "None"
        if typ.kind == TypeKind.CUSTOM_OBJECT or typ.kind == TypeKind.PARTIAL_OBJECT:
            # nested object
            if not typ.is_list:
                return self.render_custom_object(cast(CustomObject, value))
            else:
                return f"[{', '.join(self.render_custom_object(cast(CustomObject, v)) for v in cast(list, value))}]"
        else:
            # scalar
            if not typ.is_list:
                return self.render_value_scalar(cast("ScalarValue", value), typ)
            else:
                return f"[{', '.join(self.render_value_scalar(v, typ) for v in cast(list, value))}]"

    def _render_kwargs(self, **kwargs: Any) -> str:
        """Renders kwargs into a string."""
        return ", ".join(f"{k}={v}" for k, v in kwargs.items())

    def _render_args(self, *args: Any) -> str:
        """Renders args into a string."""
        return ", ".join(a for a in args if a is not None)

    def render_builtin_object(self, obj: BuiltinObject) -> str:
        """Renders the given object into an expression (incl. inlined children for node)."""
        renderer = _get_renderer(obj.metatype)
        return renderer.render(self, obj)

    def render_object(self, obj: BuiltinObject | CustomObject):
        """Renders the given objects to a Python expression."""
        if isinstance(obj, CustomObject):
            return self.render_custom_object(obj)
        elif isinstance(obj, BuiltinObject):
            return self.render_builtin_object(obj)
        else:
            assert_never(obj)

    def render_expression(
        self,
        value: BuiltinObject | CustomObject | Property,
        as_ref: bool = False,
        format: bool = False,
    ) -> str:
        """Renders a value into an expression."""
        if isinstance(value, CustomObject):
            rendered = self.render_custom_object(value)
        elif isinstance(value, BuiltinObject):
            if isinstance(value, Node) and as_ref:
                rendered = self.render_node_ref(value)
            else:
                rendered = self.render_builtin_object(value)
        elif isinstance(value, Property):
            rendered = self.render_property_ref(value)
        else:
            assert_never(value)
        if format:
            rendered = format_code(rendered)
        return rendered

    def _get_parent_child_key(self, node: Node) -> str | None:
        """Gets the 'key' for the NodeList of the given Node's parent."""
        if node.parent_ptr and node.parent_ptr in self.aliasing:
            parent_alias = self.aliasing.get(node.parent_ptr)
            parent_cls = NODE_CLASS_BY_TYPE[node.parent_ptr.node_type]
            parent_child_prop = parent_cls.get_child_property_or_error(node.metatype)
            return f"{parent_alias}.{parent_child_prop.name}"
        return None

    def render_statement(self, *nodes: Node, format: bool = False) -> str:
        """Renders the given objects to a Python block that defines those objects."""
        # render
        rendered_objs: list[str] = []
        current_children: list[str] = []
        for i, node in enumerate(nodes):
            rendered = self.render_object(node)
            node_alias = self.aliasing.get_or_add(node)
            assert node_alias is not None, f"no alias for {node!r}"
            rendered_objs.append(f"{node_alias} = {rendered}")

            # append to parent
            parent_key = self._get_parent_child_key(node)
            next_parent_key = (
                self._get_parent_child_key(nodes[i + 1]) if i < len(nodes) - 1 else None
            )
            if parent_key is not None:
                if node.metatype == NodeType.LINK:
                    continue  # implicitly added into parent (see LinkRenderer)
                current_children.append(node_alias)
                if parent_key != next_parent_key:
                    if len(current_children) > 1:
                        rendered_objs.append(f"{parent_key}.extend({', '.join(current_children)})")
                    else:
                        rendered_objs.append(f"{parent_key}.append({node_alias})")
                    current_children = []
        rendered = self.options.statement_separator.join(rendered_objs)
        if format:
            rendered = format_code(rendered)
        return rendered


def _deconstruct_builtin_object(
    obj: BuiltinObject, *, include_defaults: bool = False
) -> dict[Property, Any]:
    """Gets the 'content' values for a BuiltinObject."""
    cls = obj._get_effective_cls()
    kwargs: dict[Property, Any] = {}
    # properties
    for prop in cls.__properties__.values():
        if (
            prop.id is None
            or prop.id < 30
            or prop.reference_source
            or prop.is_value_packed
            or prop.name == "order_key"
        ):
            continue  # ignore internal properties
        prop_value = getattr(obj, prop.name)
        if (
            (prop_value is None and prop.default is None)
            or (isinstance(prop_value, Collection) and len(prop_value) == 0)
            or (prop_value is prop.default and not include_defaults)
        ):
            continue
        kwargs[prop] = prop_value
    # values last (may depend on types, and to simplify control flow for skipping unset values)
    for prop in cls.__value_runtime_properties__.values():
        wired_prop = prop.value_packed_ptr
        assert type(wired_prop) is Property, f"no wired prop for {prop!r}"
        wired_prop_value = getattr(obj, wired_prop.name)
        if wired_prop_value is None:
            continue
        prop_value = getattr(obj, prop.name)
        if prop_value is None:
            continue
        kwargs[prop] = prop_value
    return kwargs


def _render_builtin_object_kwargs(
    renderer: Renderer, obj: BuiltinObject, kwargs: dict[Property, Any]
) -> dict[str, str]:
    rendered_kwargs: dict[str, str] = {}
    for prop, value in kwargs.items():
        if prop.is_value_runtime:
            value_type = prop.value_type_info_getter(obj) if prop.value_type_info_getter else None
            if value_type is not None:
                rendered_kwargs[prop.name] = renderer.render_value(value, value_type)
        else:
            rendered_kwargs[prop.name] = renderer.render_value(value, prop.type_info)
    return rendered_kwargs


def _deconstruct_custom_object(obj: CustomObject) -> dict[Property | Field, Any]:
    """Gets the 'content' values for a CustomObject."""
    kwargs: dict[Property | Field, Any] = {}
    # properties
    for prop in get_custom_object_properties(obj._type, obj._value):
        storage_key = prop.subtype_key or prop.key
        prop_value = cast(SomeValue, obj._value.get(storage_key))
        if prop_value is None:
            continue
        kwargs[prop] = prop_value
    # fields
    for field in obj._type._fields:
        field_value = obj._do_get(field)
        if field_value is None:
            continue
        kwargs[field] = field_value
    return kwargs


def _render_custom_object_kwargs(
    renderer: "Renderer", obj: CustomObject, kwargs: dict[Property | Field, Any]
) -> dict[str, str]:
    rendered_kwargs: dict[str, str] = {}
    for prop, value in kwargs.items():
        if type(prop) is Property and prop.is_value_packed:
            assert type(prop.value_runtime_ptr) is Property, f"no runtime ptr for {prop!r}"
            rendered_kwargs[prop.value_runtime_ptr.name] = renderer.render_custom_object(value)
        else:
            rendered_kwargs[prop.name] = renderer.render_value(value, prop.type_info)
    return rendered_kwargs


def _deconstruct_type_in(
    renderer: "Renderer", obj: TypeBase, kwargs: dict[str, Any]
) -> tuple[str | None, dict[str, Any]]:
    """Remaps a Type to its TypeIn for rendering."""
    # remap back to type in if possible
    type_in = reverse_type_scalar(obj)
    if type_in is None:
        return None, kwargs
    if isinstance(type_in, Node):
        rendered_type = renderer.render_node_ref(type_in)
    elif isinstance(type_in, Enum):
        rendered_type = f"{type_in.__class__.__name__}.{type_in.name}"
    else:
        assert isinstance(type_in, type), f"unexpected type {type_in!r}"
        rendered_type = type_in.__name__
    kwargs = {**kwargs}
    for key in ("kind", "primitive_type", "bench_type", "base_type"):
        kwargs.pop(key, None)
    if isinstance(type_in, TypeFormat):
        kwargs.pop("format", None)
    return rendered_type, kwargs


def _desconstruct_partial_type(renderer: "Renderer", obj: TypeBase):
    kwargs = {}
    if obj.bench_type is not None:
        node_cls = NODE_CLASS_BY_TYPE[cast(NodeType, obj.bench_type)]
    else:
        node_cls = Node
    if obj.constraint is not None and obj.constraint.node_subtypes:
        kwargs["type"] = obj.constraint.node_subtypes[0]
    if obj.base_type is not None:
        kwargs["block"] = renderer.render_node_ref(obj.base_type)
    if obj.base_field_types:
        kwargs["field_types"] = (
            f"[{', '.join(f'FieldType.{field_type.name}' for field_type in obj.base_field_types)}]"
        )
    return node_cls, kwargs


#
# Base renderers
#


_renderers: dict[ObjectType, "BuiltinObjectRenderer"] = {}


def _renderer(object_type: ObjectType):
    """Decorator to register a Rewriter for a specific ObjectType."""

    def decorator(cls):
        if object_type in _renderers:
            raise RuntimeError(f"rewriter for {object_type!r} already registered")
        _renderers[object_type] = cls()

    return decorator


def _get_renderer(object_type: ObjectType) -> "BuiltinObjectRenderer":
    renderer = _renderers.get(object_type)
    if renderer is None:
        if is_node_type(object_type):
            node_type = NodeType(object_type)
            if node_type.is_source:
                renderer = SOURCE_NODE_RENDERER
            elif node_type.is_resource:
                renderer = RESOURCE_NODE_RENDERER
            else:
                renderer = NODE_RENDERER
        else:
            renderer = BUILTIN_OBJECT_RENDERER
    return renderer


class BuiltinObjectRenderer[T: BuiltinObject]:
    """The base renderer for a BuiltinObject."""

    def render(self, renderer: "Renderer", obj: T) -> str:
        """Render the given object to a Python expression (string)."""
        kwargs = _deconstruct_builtin_object(obj, include_defaults=False)
        rendered_kwargs = _render_builtin_object_kwargs(renderer, obj, kwargs)
        return f"{obj.__class__.__name__}({renderer._render_kwargs(**rendered_kwargs)})"


#
# Node renderers
#


class NodeRenderer[T: Node](BuiltinObjectRenderer[T]):
    """The base renderer for a Node."""

    def _render_child_properties(
        self, renderer: "Renderer", obj: T, rendered_kwargs: dict[str, str] | None = None
    ) -> dict[str, str]:
        rendered_kwargs = rendered_kwargs if rendered_kwargs is not None else {}
        for prop in obj.__node_child_properties__.values():
            assert prop.reference_nodes, f"no reference nodes for {prop!r}"
            if prop.reference_nodes[0] not in renderer.options.inline_node_types:
                continue
            children = getattr(obj, prop.name)
            if not children:
                continue
            rendered_children = [
                renderer.render_builtin_object(cast(BuiltinObject, child)) for child in children
            ]
            rendered_kwargs[prop.name] = f"[{', '.join(rendered_children)}]"
        return rendered_kwargs

    def _render_constructor(
        self,
        renderer: "Renderer",
        obj: T,
        kwargs: dict[Property, Any],
        rendered_kwargs: dict[str, str],
    ) -> str:
        """Create the constructor expression for a BuiltinObject (for the default .render)."""
        return f"{obj.__class__.__name__}({renderer._render_kwargs(**rendered_kwargs)})"

    @override
    def render(self, renderer: Renderer, obj: T) -> str:
        kwargs = _deconstruct_builtin_object(obj, include_defaults=False)
        rendered_kwargs = _render_builtin_object_kwargs(renderer, obj, kwargs)
        rendered_kwargs = self._render_child_properties(renderer, obj, rendered_kwargs)
        return self._render_constructor(renderer, obj, kwargs, rendered_kwargs)


class SourceNodeRenderer[T: SourceNode](NodeRenderer[T]):
    """The base renderer for a SourceNode."""

    @override
    def render(self, renderer: Renderer, obj: T) -> str:
        kwargs = _deconstruct_builtin_object(obj, include_defaults=False)
        if computed_values := obj.computed_values:
            kwargs[obj.get_property("computed_values")] = computed_values
        rendered_kwargs = _render_builtin_object_kwargs(renderer, obj, kwargs)
        rendered_kwargs = self._render_child_properties(renderer, obj, rendered_kwargs)
        return self._render_constructor(renderer, obj, kwargs, rendered_kwargs)


class ResourceNodeRenderer[T: Resource](NodeRenderer[T]):
    """The base renderer for a ResourceNode."""


NODE_RENDERER = NodeRenderer[Node]()
SOURCE_NODE_RENDERER = SourceNodeRenderer[SourceNode]()
RESOURCE_NODE_RENDERER = ResourceNodeRenderer[Resource]()
BUILTIN_OBJECT_RENDERER = BuiltinObjectRenderer[BuiltinObject]()


@_renderer(NodeType.BLOCK)
class BlockRenderer(SourceNodeRenderer[Block]):
    @override
    def _render_constructor(
        self,
        renderer: "Renderer",
        obj: Block,
        kwargs: dict[Property, Any],
        rendered_kwargs: dict[str, str],
    ) -> str:
        block_args = renderer._render_args(
            rendered_kwargs.pop("type"),
            rendered_kwargs.pop("name"),
            renderer._render_kwargs(**rendered_kwargs) or None,
        )
        return f"Block.new({block_args})"


@_renderer(NodeType.VIEW)
class ViewRenderer(SourceNodeRenderer[View]):
    @override
    def _render_constructor(
        self,
        renderer: "Renderer",
        obj: View,
        kwargs: dict[Property, Any],
        rendered_kwargs: dict[str, str],
    ) -> str:
        view_args = renderer._render_args(
            rendered_kwargs.pop("type"),
            rendered_kwargs.pop("name"),
            renderer._render_kwargs(**rendered_kwargs) or None,
        )
        return f"View.new({view_args})"


@_renderer(NodeType.FLOW)
class FlowRenderer(SourceNodeRenderer[Flow]):
    @override
    def _render_constructor(
        self,
        renderer: "Renderer",
        obj: Flow,
        kwargs: dict[Property, Any],
        rendered_kwargs: dict[str, str],
    ) -> str:
        # inline name only for now
        args = (
            rendered_kwargs.pop("name"),
            renderer._render_kwargs(**rendered_kwargs) or None,
        )
        return f"Flow.new({renderer._render_args(*args)})"


@_renderer(NodeType.ACTION)
class ActionRenderer(SourceNodeRenderer[Action]):
    @override
    def _render_constructor(
        self,
        renderer: "Renderer",
        obj: Action,
        kwargs: dict[Property, Any],
        rendered_kwargs: dict[str, str],
    ) -> str:
        action_args = renderer._render_args(
            rendered_kwargs.pop("type"),
            rendered_kwargs.pop("name"),
            renderer._render_kwargs(**rendered_kwargs) or None,
        )
        return f"Action.new({action_args})"


@_renderer(NodeType.LINK)
class LinkRenderer(SourceNodeRenderer[Link]):
    @override
    def _render_constructor(
        self,
        renderer: "Renderer",
        obj: Link,
        kwargs: dict[Property, Any],
        rendered_kwargs: dict[str, str],
    ) -> str:
        if (
            (obj.parent) is not None
            and (source := obj.source) is not None
            and (target := obj.target) is not None
        ):
            source_ref = renderer.render_node_ref(source)
            target_ref = renderer.render_node_ref(target)
            rendered_kwargs.pop("type", None)
            rendered_kwargs.pop("source", None)
            rendered_kwargs.pop("target", None)
            rendered_kwargs.pop("name", None)
            args = (
                f"LinkType.{obj.type.name}",
                target_ref,
                repr(obj.name),
                renderer._render_kwargs(**rendered_kwargs) or None,
            )
            return f"{source_ref}.connect({renderer._render_args(*args)})"
        else:
            args = (
                rendered_kwargs.pop("type"),
                rendered_kwargs.pop("name"),
                renderer._render_kwargs(**rendered_kwargs) or None,
            )
            return f"Link.new({renderer._render_args(*args)})"


@_renderer(NodeType.CHOICE)
class ChoiceRenderer(SourceNodeRenderer[Choice]):
    @override
    def _render_constructor(
        self,
        renderer: "Renderer",
        obj: Choice,
        kwargs: dict[Property, Any],
        rendered_kwargs: dict[str, str],
    ) -> str:
        # inline name and fields (like Choice.new(name, *fields))
        options_refs = [renderer.render_builtin_object(option) for option in obj.options]
        rendered_kwargs.pop("options", None)
        args = (
            rendered_kwargs.pop("name"),
            *options_refs,
            renderer._render_kwargs(**rendered_kwargs) or None,
        )
        return f"Choice.new({renderer._render_args(*args)})"


@_renderer(NodeType.CLASS)
class ClassRenderer(SourceNodeRenderer[Class]):
    @override
    def _render_constructor(
        self,
        renderer: "Renderer",
        obj: Class,
        kwargs: dict[Property, Any],
        rendered_kwargs: dict[str, str],
    ) -> str:
        # inline name and fields (like Class.new(name, *fields))
        fields_refs = [renderer.render_builtin_object(field) for field in obj.fields]
        rendered_kwargs.pop("fields", None)
        args = (
            rendered_kwargs.pop("name"),
            *fields_refs,
            renderer._render_kwargs(**rendered_kwargs) or None,
        )
        return f"Class.new({renderer._render_args(*args)})"


@_renderer(NodeType.DATABASE)
class DatabaseRenderer(SourceNodeRenderer[Database]):
    @override
    def _render_constructor(
        self,
        renderer: "Renderer",
        obj: Database,
        kwargs: dict[Property, Any],
        rendered_kwargs: dict[str, str],
    ) -> str:
        # inline name and fields (like Database.new(name, *fields))
        fields_refs = [renderer.render_builtin_object(field) for field in obj.fields]
        rendered_kwargs.pop("fields", None)
        args = (
            rendered_kwargs.pop("name"),
            *fields_refs,
            renderer._render_kwargs(**rendered_kwargs) or None,
        )
        return f"Database.new({renderer._render_args(*args)})"


@_renderer(NodeType.PAGE)
class PageRenderer(SourceNodeRenderer[Page]):
    @override
    def _render_constructor(
        self,
        renderer: "Renderer",
        obj: Page,
        kwargs: dict[Property, Any],
        rendered_kwargs: dict[str, str],
    ) -> str:
        # inline name only for now
        args = (
            rendered_kwargs.pop("name"),
            renderer._render_kwargs(**rendered_kwargs) or None,
        )
        return f"Page.new({renderer._render_args(*args)})"


@_renderer(NodeType.FIELD)
class FieldRenderer(SourceNodeRenderer[Field]):
    @override
    def _render_constructor(
        self,
        renderer: "Renderer",
        obj: Field,
        kwargs: dict[Property, Any],
        rendered_kwargs: dict[str, str],
    ) -> str:
        # remap back to type in if possible
        type_in, rendered_kwargs = _deconstruct_type_in(renderer, obj, rendered_kwargs)
        # is_required=True is implicit if variable
        if obj.type == FieldType.VARIABLE and obj.is_required is True:
            rendered_kwargs.pop("is_required", None)

        constructor_name = obj.type.name.lower()
        rendered_kwargs.pop("type", None)
        if type_in is not None:
            field_args = renderer._render_args(
                rendered_kwargs.pop("name"),
                type_in,
                renderer._render_kwargs(**rendered_kwargs) or None,
            )
        else:
            field_args = renderer._render_args(
                rendered_kwargs.pop("name"), renderer._render_kwargs(**rendered_kwargs) or None
            )
        return f"Field.{constructor_name}({field_args})"


@_renderer(NodeType.OPTION)
class OptionRenderer(SourceNodeRenderer[Option]):
    @override
    def _render_constructor(
        self,
        renderer: "Renderer",
        obj: Option,
        kwargs: dict[Property, Any],
        rendered_kwargs: dict[str, str],
    ) -> str:
        # inline name only for now
        args = (
            rendered_kwargs.pop("name"),
            renderer._render_kwargs(**rendered_kwargs) or None,
        )
        return f"Option.new({renderer._render_args(*args)})"


#
# Struct renderers
#


@_renderer(StructType.TYPE)
class TypeRenderer(BuiltinObjectRenderer[TypeBase]):
    @override
    def render(self, renderer: "Renderer", obj: TypeBase) -> str:
        if obj.kind == TypeKind.PARTIAL_OBJECT:
            node_cls, rendered_kwargs = _desconstruct_partial_type(renderer, obj)
            return f"{node_cls.__name__}.partial_type({renderer._render_kwargs(**rendered_kwargs)})"
        kwargs = _deconstruct_builtin_object(obj, include_defaults=False)
        rendered_kwargs = _render_builtin_object_kwargs(renderer, obj, kwargs)
        type_in, rendered_kwargs = _deconstruct_type_in(renderer, obj, rendered_kwargs)
        if type_in is not None:
            type_args = renderer._render_args(
                type_in, renderer._render_kwargs(**rendered_kwargs) or None
            )
        else:
            type_args = renderer._render_args(renderer._render_kwargs(**rendered_kwargs) or None)
        return f"to_type({type_args})"


@_renderer(StructType.TYPE_CONSTRAINT)
class TypeConstraintRenderer(BuiltinObjectRenderer[TypeConstraint]):
    @override
    def render(self, renderer: "Renderer", obj: TypeConstraint) -> str:
        kwargs = _deconstruct_builtin_object(obj, include_defaults=False)
        rendered_kwargs = _render_builtin_object_kwargs(renderer, obj, kwargs)
        return f"constraint({renderer._render_kwargs(**rendered_kwargs)})"


@_renderer(StructType.TEXT)
class TextRenderer(BuiltinObjectRenderer[Text]):
    @override
    def render(self, renderer: "Renderer", obj: Text) -> str:
        return f"text({text_to_markdown(obj, renderer.aliasing)!r})"


@_renderer(StructType.CODE)
class CodeRenderer(BuiltinObjectRenderer[Code]):
    @override
    def render(self, renderer: "Renderer", obj: Code) -> str:
        return f"code({obj.to_string()!r})"


@_renderer(StructType.ICON)
class IconRenderer(BuiltinObjectRenderer[Icon]):
    @override
    def render(self, renderer: "Renderer", obj: Icon) -> str:
        simplified = reverse_icon(obj)
        if isinstance(simplified, str):
            return f"icon({simplified!r})"
        else:
            return super().render(renderer, obj)


def _render_path_element_in(renderer: "Renderer", element: PathElement) -> str:
    element_in = reverse_path_element(element)
    if isinstance(element_in, Node):
        return renderer.render_node_ref(element_in)
    elif isinstance(element_in, Property):
        return renderer.render_property_ref(element_in)
    elif isinstance(element_in, PathElementType):
        return f"PathElementType.{element_in.name}"
    elif isinstance(element_in, str):
        return element_in
    elif isinstance(element_in, PathElement):
        return renderer.render_expression(element_in)
    else:
        assert_never(element_in)


def _render_path_in(renderer: "Renderer", obj: Path) -> str:
    elements_in: list[str] = []
    for element in obj.elements:
        element_in_str = _render_path_element_in(renderer, element)
        elements_in.append(element_in_str)
    return f"({', '.join(elements_in)})"


@_renderer(StructType.PATH_ELEMENT)
class PathElementRenderer(BuiltinObjectRenderer[PathElement]):
    @override
    def render(self, renderer: "Renderer", obj: PathElement) -> str:
        element_in_str = _render_path_element_in(renderer, obj)
        return f"path_element({element_in_str})"


@_renderer(StructType.PATH)
class PathRenderer(BuiltinObjectRenderer[Path]):
    @override
    def render(self, renderer: "Renderer", obj: Path) -> str:
        path_in_str = _render_path_in(renderer, obj)
        return f"path{path_in_str}"


@_renderer(StructType.COMPUTED_VALUE)
class ComputedValueRenderer(BuiltinObjectRenderer[ComputedValue]):
    @override
    def render(self, renderer: "Renderer", obj: ComputedValue) -> str:
        rendered_kwargs = {}
        if target := obj.target_path:
            rendered_kwargs["target"] = _render_path_in(renderer, target)
        if (source := obj.source) is not None:
            if isinstance(source, Path):
                rendered_kwargs["source"] = _render_path_in(renderer, source)
            else:
                rendered_kwargs["source"] = renderer.render_expression(source)
        if obj.mode != ComputedValueMode.ALWAYS:
            rendered_kwargs["mode"] = f"ComputedValueMode.{obj.mode.name}"
        return f"ComputedValue.new({renderer._render_kwargs(**rendered_kwargs)})"


def _render_call_in(renderer: "Renderer", obj: Call) -> str:
    kwargs = _deconstruct_builtin_object(obj, include_defaults=False)
    node = kwargs.pop(Call.get_property("node"))
    node_str = renderer.render_node_ref(node)
    value = kwargs.pop(Call.get_property("value"))
    value_kwargs = _deconstruct_custom_object(value)
    # render
    rendered_kwargs = _render_builtin_object_kwargs(renderer, obj, kwargs)
    rendered_value_kwargs = _render_custom_object_kwargs(renderer, value, value_kwargs)
    rendered_kwargs.update(rendered_value_kwargs)
    args = renderer._render_args(node_str, renderer._render_kwargs(**rendered_kwargs) or None)
    return args


@_renderer(StructType.CALL)
class CallRenderer(BuiltinObjectRenderer[Call]):
    @override
    def render(self, renderer: "Renderer", obj: Call) -> str:
        return f"call({_render_call_in(renderer, obj)})"


@_renderer(StructType.CALL_PLAN)
class CallPlanRenderer(BuiltinObjectRenderer[CallPlan]):
    @override
    def render(self, renderer: "Renderer", obj: CallPlan) -> str:
        if not obj.calls:
            return "call_none()"
        # defaults
        kwargs = _deconstruct_builtin_object(obj, include_defaults=False)
        kwargs.pop(CallPlan.get_property("execution"), None)
        calls = kwargs.pop(CallPlan.get_property("calls"), ())
        if kwargs.get(CallPlan.get_property("on_error")) == CallFailureMode.FAIL:
            kwargs.pop(CallPlan.get_property("on_error"), None)
        if kwargs.get(CallPlan.get_property("on_terminate")) == CallTerminationMode.PASS:
            kwargs.pop(CallPlan.get_property("on_terminate"), None)
        # execution mode
        if obj.execution == CallExecutionMode.PARALLEL:
            func = "call_parallel"
        elif obj.execution == CallExecutionMode.SERIAL:
            func = "call_serial"
        else:
            assert_never(obj.execution)
        rendered_kwargs = _render_builtin_object_kwargs(renderer, obj, kwargs)
        calls_strs = [f"call({_render_call_in(renderer, call)})" for call in calls]
        args = renderer._render_args(
            *calls_strs, renderer._render_kwargs(**rendered_kwargs) or None
        )
        return f"{func}({args})"


#
# Top level helpers
#


def render_value(value: SomeValue, typ: TypeBase, options: RenderOptions) -> str:
    """Render the given value to a python expression."""
    renderer = Renderer(options)
    rendered = renderer.render_value(value, typ)
    return rendered


def render_expression(
    value: BuiltinObject | CustomObject | Property, options: RenderOptions, as_ref: bool = False
) -> str:
    """Render the given object to a python expression."""
    renderer = Renderer(options)
    rendered = renderer.render_expression(value, as_ref=as_ref)
    if options.format:
        rendered = format_code(rendered, line_length=options.line_length)
    return rendered.strip()


def render_expressions(
    expressions: Mapping[str, BuiltinObject | CustomObject | Property],
    options: RenderOptions,
) -> str:
    """Render the given expressions to python expressions."""
    renderer = Renderer(options)
    return options.statement_separator.join(
        f"{name} = {renderer.render_expression(value, as_ref=True)}"
        for name, value in expressions.items()
    )


def render_statement(*objs: Node, options: RenderOptions) -> str:
    """Renders the given object to a python block where the objects are defined."""
    renderer = Renderer(options)
    for obj in objs:
        renderer.aliasing.add(obj)
    rendered = renderer.render_statement(*objs)
    if options.format:
        rendered = format_code(rendered, line_length=options.line_length)
    return rendered.strip()


@overload
def render(*objs: BuiltinObject, options: RenderOptions) -> str: ...
@overload
def render(*objs: CustomObject | Property, options: RenderOptions) -> str: ...
def render(*objs: BuiltinObject | CustomObject | Property, options: RenderOptions) -> str:
    """Renders the given object to either an expression (for values) or statement (for nodes)."""
    if any(isinstance(obj, Node) for obj in objs):
        return render_statement(*cast(list[Node], objs), options=options)
    else:
        # render into tuple of expressions
        value_exprs = [render_expression(obj, options) for obj in objs]
        return ", ".join(value_exprs)
