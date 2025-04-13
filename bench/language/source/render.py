import base64
import contextvars
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
    CustomObject,
    EnumType,
    FieldType,
    Icon,
    IsModal,
    IsType,
    Node,
    NodeMode,
    NodeReference,
    NodeType,
    ObjectType,
    PackageNode,
    PrimitiveType,
    Property,
    PropertyReference,
    Resource,
    ScalarValue,
    SomeValue,
    Struct,
    StructType,
    Text,
    TextLine,
    TypeConstraint,
    TypeFormat,
    TypeKind,
    format_code,
    get_custom_object_properties,
    is_node_type,
    reverse_icon,
    reverse_type_scalar,
    text_line_to_markdown,
    text_to_markdown,
)
from bench.language.core.const import ReferenceKind
from bench.language.registry import ENUM_CLASS_BY_TYPE, NODE_CLASS_BY_TYPE
from bench.utils.time import timedelta_to_isoformat

from .action import Action
from .block import Block
from .choice import Choice
from .clazz import Class
from .database import Database
from .field import Field
from .flow import Flow
from .link import Transition
from .option import Option
from .page import Page

if TYPE_CHECKING:
    from bench.language import Claim, Plan, Task, View

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


@dataclass(slots=True)
class RenderOptions:
    scope: Node
    aliasing: "Aliasing"
    node_types: Collection[NodeType] = NODE_TYPES_SET
    inline_node_types: Collection[NodeType] = (NodeType.FIELD, NodeType.OPTION, NodeType.TRIGGER)
    use_code_paths: bool = True
    implicit_partials: bool = True
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

    def clone(self) -> "Aliasing":
        """Clone the current aliasing registry."""
        aliasing = Aliasing()
        aliasing._alias_by_node_id = self._alias_by_node_id.copy()
        aliasing._node_by_alias = self._node_by_alias.copy()
        return aliasing

    def add(self, obj: Node | NodeReference) -> str:
        """Adds the given nodes to the context of this renderer."""
        from bench.runtime.code.context import PYTHON_KEYWORDS, STATIC_CODE_GLOBALS

        if obj.id in self._alias_by_node_id:
            return self._alias_by_node_id[obj.id]  # already assigned
        if isinstance(obj, Node):
            if code_name := getattr(obj, "code_name", None):
                # proper given name (should be valid python identifier)
                alias = code_name
                if not regex.match(r"^[a-zA-Z_]\w+$", alias):
                    alias = f"{obj.metatype.bench_name}_{alias}"
                has_given_name = True
            else:
                alias = obj.metatype.bench_name
                has_given_name = False
        else:
            alias = obj.node_type.bench_name
            has_given_name = False
        if (
            alias in self._node_by_alias
            or alias in STATIC_CODE_GLOBALS
            or alias in PYTHON_KEYWORDS
            or not has_given_name
        ):
            # bump digit at end to make alias unique
            count = regex.search(r"\d+$", alias)
            if count is None:
                alias = f"{alias}1"
                count = 1
            else:
                count = int(count.group())
            while (
                alias in self._node_by_alias
                or alias in STATIC_CODE_GLOBALS
                or alias in PYTHON_KEYWORDS
            ):
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


ACTIVE_ALIASING: contextvars.ContextVar[Aliasing | None] = contextvars.ContextVar(
    "active_aliasing", default=None
)


def get_active_aliasing() -> Aliasing | None:
    return ACTIVE_ALIASING.get()


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

        # otherwise just make context-specific alias
        alias = self.aliasing.get_or_add(node)
        return alias

    def render_property_ref(self, prop: Property | PropertyReference) -> str:
        if isinstance(prop, PropertyReference):
            prop = prop.resolve_or_error()
        if prop.value_runtime_ptr is not None:
            prop = prop.value_runtime_ptr
        return f'{prop.component.__name__}.get_property("{prop.name}")'

    def render_value_scalar(self, value: "ScalarValue", typ: "IsType") -> str:
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
                    self.render_kwargs(**rendered_kwargs) or None,
                )
                return f"{node_cls.__name__}.partial({self.render_args(*args)})"
            else:
                return f"{node_cls.__name__}.partial({self.render_kwargs(**rendered_kwargs)})"
        elif (
            typ.base_field_types
            and FieldType.MEMBER in typ.base_field_types
            and (base_type := typ.base_type) is not None
        ):
            # object representation
            kwargs_str = f"{base_type.code_name}({self.render_kwargs(**rendered_kwargs)})"
            return kwargs_str
        else:
            # default dict representation
            kwargs_str = ", ".join(f"'{k}': {v}" for k, v in rendered_kwargs.items())
            kwargs_str = f"{{{', '.join({kwargs_str})}}}"
            return kwargs_str

    def render_value(self, value: "SomeValue | None", typ: "IsType") -> str:
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

    def render_kwargs(self, **kwargs: Any) -> str:
        """Renders kwargs into a string."""
        return ", ".join(f"{k}={v}" for k, v in kwargs.items())

    def render_args(self, *args: Any) -> str:
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
            parent_child_prop = parent_cls.get_child_property(node.metatype)
            if parent_child_prop is not None:
                return f"{parent_alias}.{parent_child_prop.name}"
        return None

    def render_statement(self, *nodes: Node, append: bool = True, format: bool = False) -> str:
        """Renders the given objects to a Python block that defines those objects."""
        # render
        rendered_objs: list[str] = []
        current_children: list[str] = []
        for i, node in enumerate(nodes):
            rendered = self.render_builtin_object(node)
            node_alias = self.aliasing.get_or_add(node)
            assert node_alias is not None, f"no alias for {node!r}"
            rendered_objs.append(f"{node_alias} = {rendered}")

            # append to parent
            if append:
                parent_key = self._get_parent_child_key(node)
                next_parent_key = (
                    self._get_parent_child_key(nodes[i + 1]) if i < len(nodes) - 1 else None
                )
                if parent_key is not None:
                    if node.metatype == NodeType.TRANSITION:
                        continue  # implicitly added into parent (see LinkRenderer)
                    current_children.append(node_alias)
                    if parent_key != next_parent_key:
                        if len(current_children) > 1:
                            rendered_objs.append(
                                f"{parent_key}.extend({', '.join(current_children)})"
                            )
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
            or (prop.id < 30 and prop.name != "created_by")
            or prop.reference_source
            or prop.reference_kind == ReferenceKind.NODE_ANCESTOR
            or prop.reference_kind == ReferenceKind.NODE_ANCESTOR_OR_SELF
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
    if isinstance(obj, IsModal) and obj.mode != NodeMode.MAIN:
        kwargs[obj.get_property("mode")] = obj.mode
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
        elif name := prop.name:
            rendered_kwargs[name] = renderer.render_value(value, prop.type_info)
    return rendered_kwargs


def _deconstruct_type_in(
    renderer: "Renderer", obj: IsType, kwargs: dict[str, Any]
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


def _desconstruct_partial_type(renderer: "Renderer", obj: IsType):
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
                renderer = PACKAGE_NODE_RENDERER
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
        return f"{obj.__class__.__name__}({renderer.render_kwargs(**rendered_kwargs)})"


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
        return f"{obj.__class__.__name__}({renderer.render_kwargs(**rendered_kwargs)})"

    @override
    def render(self, renderer: Renderer, obj: T) -> str:
        kwargs = _deconstruct_builtin_object(obj, include_defaults=False)
        rendered_kwargs = _render_builtin_object_kwargs(renderer, obj, kwargs)
        rendered_kwargs = self._render_child_properties(renderer, obj, rendered_kwargs)
        return self._render_constructor(renderer, obj, kwargs, rendered_kwargs)


class PackageNodeRenderer[T: PackageNode](NodeRenderer[T]):
    """The base renderer for a PackageNode."""

    @override
    def render(self, renderer: Renderer, obj: T) -> str:
        kwargs = _deconstruct_builtin_object(obj, include_defaults=False)
        rendered_kwargs = _render_builtin_object_kwargs(renderer, obj, kwargs)
        rendered_kwargs = self._render_child_properties(renderer, obj, rendered_kwargs)
        return self._render_constructor(renderer, obj, kwargs, rendered_kwargs)


class ResourceNodeRenderer[T: Resource](NodeRenderer[T]):
    """The base renderer for a ResourceNode."""

    @override
    def _render_constructor(
        self,
        renderer: Renderer,
        obj: T,
        kwargs: dict[Property, Any],
        rendered_kwargs: dict[str, str],
    ) -> str:
        view_args = renderer.render_args(
            rendered_kwargs.pop("type"),
            rendered_kwargs.pop("name"),
            renderer.render_kwargs(**rendered_kwargs) or None,
        )
        return f"{obj.__class__.__name__}.new({view_args})"


NODE_RENDERER = NodeRenderer[Node]()
PACKAGE_NODE_RENDERER = PackageNodeRenderer[PackageNode]()
RESOURCE_NODE_RENDERER = ResourceNodeRenderer[Resource]()
BUILTIN_OBJECT_RENDERER = BuiltinObjectRenderer[BuiltinObject]()


@_renderer(NodeType.BLOCK)
class BlockRenderer(PackageNodeRenderer[Block]):
    @override
    def _render_constructor(
        self,
        renderer: "Renderer",
        obj: Block,
        kwargs: dict[Property, Any],
        rendered_kwargs: dict[str, str],
    ) -> str:
        block_args = renderer.render_args(
            rendered_kwargs.pop("type"),
            rendered_kwargs.pop("name"),
            renderer.render_kwargs(**rendered_kwargs) or None,
        )
        return f"Block.new({block_args})"


@_renderer(NodeType.VIEW)
class ViewRenderer(PackageNodeRenderer["View"]):
    @override
    def _render_constructor(
        self,
        renderer: "Renderer",
        obj: "View",
        kwargs: dict[Property, Any],
        rendered_kwargs: dict[str, str],
    ) -> str:
        view_args = renderer.render_args(
            rendered_kwargs.pop("type"),
            rendered_kwargs.pop("name"),
            renderer.render_kwargs(**rendered_kwargs) or None,
        )
        return f"View.new({view_args})"


@_renderer(NodeType.FLOW)
class FlowRenderer(PackageNodeRenderer[Flow]):
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
            renderer.render_kwargs(**rendered_kwargs) or None,
        )
        return f"Flow.new({renderer.render_args(*args)})"


@_renderer(NodeType.ACTION)
class ActionRenderer(PackageNodeRenderer[Action]):
    @override
    def _render_constructor(
        self,
        renderer: "Renderer",
        obj: Action,
        kwargs: dict[Property, Any],
        rendered_kwargs: dict[str, str],
    ) -> str:
        action_args = renderer.render_args(
            rendered_kwargs.pop("type"),
            rendered_kwargs.pop("name"),
            renderer.render_kwargs(**rendered_kwargs) or None,
        )
        return f"Action.new({action_args})"


@_renderer(NodeType.TRANSITION)
class LinkRenderer(PackageNodeRenderer[Transition]):
    @override
    def _render_constructor(
        self,
        renderer: "Renderer",
        obj: Transition,
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
                renderer.render_kwargs(**rendered_kwargs) or None,
            )
            return f"{source_ref}.connect({renderer.render_args(*args)})"
        else:
            args = (
                rendered_kwargs.pop("type"),
                rendered_kwargs.pop("name"),
                renderer.render_kwargs(**rendered_kwargs) or None,
            )
            return f"Link.new({renderer.render_args(*args)})"


@_renderer(NodeType.CHOICE)
class ChoiceRenderer(PackageNodeRenderer[Choice]):
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
            renderer.render_kwargs(**rendered_kwargs) or None,
        )
        return f"Choice.new({renderer.render_args(*args)})"


@_renderer(NodeType.CLASS)
class ClassRenderer(PackageNodeRenderer[Class]):
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
            renderer.render_kwargs(**rendered_kwargs) or None,
        )
        return f"Class.new({renderer.render_args(*args)})"


@_renderer(NodeType.DATABASE)
class DatabaseRenderer(PackageNodeRenderer[Database]):
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
            renderer.render_kwargs(**rendered_kwargs) or None,
        )
        return f"Database.new({renderer.render_args(*args)})"


@_renderer(NodeType.PAGE)
class PageRenderer(PackageNodeRenderer[Page]):
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
            rendered_kwargs.pop("title"),
            renderer.render_kwargs(**rendered_kwargs) or None,
        )
        return f"Page.new({renderer.render_args(*args)})"


@_renderer(NodeType.FIELD)
class FieldRenderer(PackageNodeRenderer[Field]):
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

        constructor_name = obj.type.name.lower()
        rendered_kwargs.pop("type", None)
        if type_in is not None:
            field_args = renderer.render_args(
                rendered_kwargs.pop("name"),
                type_in,
                renderer.render_kwargs(**rendered_kwargs) or None,
            )
        else:
            field_args = renderer.render_args(
                rendered_kwargs.pop("name"), renderer.render_kwargs(**rendered_kwargs) or None
            )
        return f"Field.{constructor_name}({field_args})"


@_renderer(NodeType.OPTION)
class OptionRenderer(PackageNodeRenderer[Option]):
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
            renderer.render_kwargs(**rendered_kwargs) or None,
        )
        return f"Option.new({renderer.render_args(*args)})"


@_renderer(NodeType.PLAN)
class PlanRenderer(NodeRenderer["Plan"]):
    @override
    def _render_constructor(
        self,
        renderer: "Renderer",
        obj: "Plan",
        kwargs: dict[Property, Any],
        rendered_kwargs: dict[str, str],
    ) -> str:
        # inline tasks like in Class.new
        tasks_refs = [renderer.render_builtin_object(task) for task in obj.tasks]
        rendered_kwargs.pop("type", None)
        rendered_kwargs.pop("tasks", None)
        args = (
            rendered_kwargs.pop("title"),
            *tasks_refs,
            renderer.render_kwargs(**rendered_kwargs) or None,
        )
        return f"Plan.new({renderer.render_args(*args)})"


@_renderer(NodeType.TASK)
class TaskRenderer(NodeRenderer["Task"]):
    @override
    def render(self, renderer: "Renderer", obj: "Task") -> str:
        from bench.language import Task

        kwargs = _deconstruct_builtin_object(obj, include_defaults=False)
        target = kwargs.pop(Task.get_property("target"))
        target_str = renderer.render_node_ref(target)
        value = kwargs.pop(Task.get_property("value"))
        value_kwargs = _deconstruct_custom_object(value)
        kwargs.pop(Task.get_property("type"), None)
        # render
        rendered_kwargs = _render_builtin_object_kwargs(renderer, obj, kwargs)
        rendered_value_kwargs = _render_custom_object_kwargs(renderer, value, value_kwargs)
        rendered_kwargs.update(rendered_value_kwargs)
        args = renderer.render_args(
            rendered_kwargs.pop("title"),
            target_str,
            renderer.render_kwargs(**rendered_kwargs) or None,
        )
        return f"Task.new({args})"


@_renderer(NodeType.CLAIM)
class ClaimRenderer(NodeRenderer["Claim"]):
    @override
    def _render_constructor(
        self,
        renderer: "Renderer",
        obj: "Claim",
        kwargs: dict[Property, Any],
        rendered_kwargs: dict[str, str],
    ) -> str:
        # inline type and name
        rendered_kwargs.pop("type", None)
        args = (
            rendered_kwargs.pop("name"),
            rendered_kwargs.pop("target"),
            renderer.render_kwargs(**rendered_kwargs) or None,
        )
        return f"Claim.{obj.type.name.lower()}({renderer.render_args(*args)})"


#
# Struct renderers
#


@_renderer(StructType.TYPE)
class TypeRenderer(BuiltinObjectRenderer[IsType]):
    @override
    def render(self, renderer: "Renderer", obj: IsType) -> str:
        if obj.kind == TypeKind.PARTIAL_OBJECT:
            node_cls, rendered_kwargs = _desconstruct_partial_type(renderer, obj)
            return f"{node_cls.__name__}.partial_type({renderer.render_kwargs(**rendered_kwargs)})"
        kwargs = _deconstruct_builtin_object(obj, include_defaults=False)
        rendered_kwargs = _render_builtin_object_kwargs(renderer, obj, kwargs)
        type_in, rendered_kwargs = _deconstruct_type_in(renderer, obj, rendered_kwargs)
        if type_in is not None:
            type_args = renderer.render_args(
                type_in, renderer.render_kwargs(**rendered_kwargs) or None
            )
        else:
            type_args = renderer.render_args(renderer.render_kwargs(**rendered_kwargs) or None)
        return f"to_type({type_args})"


@_renderer(StructType.TYPE_CONSTRAINT)
class TypeConstraintRenderer(BuiltinObjectRenderer[TypeConstraint]):
    @override
    def render(self, renderer: "Renderer", obj: TypeConstraint) -> str:
        kwargs = _deconstruct_builtin_object(obj, include_defaults=False)
        rendered_kwargs = _render_builtin_object_kwargs(renderer, obj, kwargs)
        return f"constraint({renderer.render_kwargs(**rendered_kwargs)})"


@_renderer(StructType.TEXT_LINE)
class TextLineRenderer(BuiltinObjectRenderer[TextLine]):
    @override
    def render(self, renderer: "Renderer", obj: TextLine) -> str:
        return f"text_line({text_line_to_markdown(obj, renderer.aliasing)!r})"


@_renderer(StructType.TEXT)
class TextRenderer(BuiltinObjectRenderer[Text]):
    @override
    def render(self, renderer: "Renderer", obj: Text) -> str:
        rendered_string = text_to_markdown(obj, renderer.aliasing)
        if "\n" in rendered_string:
            return f'text("""\\\n{rendered_string}\n""")'
        else:
            return f"text({rendered_string!r})"


@_renderer(StructType.CODE)
class CodeRenderer(BuiltinObjectRenderer[Code]):
    @override
    def render(self, renderer: "Renderer", obj: Code) -> str:
        rendered_string = obj.to_string()
        if "\n" in rendered_string:
            return f'code("""\\\n{rendered_string}\n""")'
        else:
            return f"code({rendered_string!r})"


@_renderer(StructType.ICON)
class IconRenderer(BuiltinObjectRenderer[Icon]):
    @override
    def render(self, renderer: "Renderer", obj: Icon) -> str:
        simplified = reverse_icon(obj)
        if isinstance(simplified, str):
            return f"icon({simplified!r})"
        else:
            return super().render(renderer, obj)


#
# Top level helpers
#


def render_value(value: SomeValue, typ: IsType, options: RenderOptions) -> str:
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
