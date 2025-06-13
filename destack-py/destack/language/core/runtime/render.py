import contextvars
import dataclasses
from collections.abc import Collection, Mapping
from dataclasses import dataclass
from typing import (
    TYPE_CHECKING,
    Any,
    assert_never,
    cast,
    overload,
    override,
)

import regex
import structlog
from fastuuid import UUID
from opentelemetry import trace

from destack.utils.code import format_code

from ..builtin import (
    NODE_TYPES,
    BuiltinObjectBase,
    Node,
    NodeType,
    Property,
    StructType,
)
from ..common import NodeReference, PropertyReference
from .graph import Supergraph

if TYPE_CHECKING:
    pass

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


@dataclass(slots=True)
class RenderOptions:
    aliasing: "Aliasing"
    include_properties: Mapping[NodeType | StructType, Collection[Property]] | None = None
    exclude_properties: Mapping[NodeType | StructType, Collection[Property]] | None = None
    node_types: Collection[NodeType] = NODE_TYPES
    # formatting
    statement_separator: str = "\n"
    format: bool = True
    line_length: int = 100

    def replace(self, **kwargs) -> "RenderOptions":
        return dataclasses.replace(self, **kwargs)


class Aliasing:
    """Registry of aliases for Nodes."""

    def __init__(self, supergraph: Supergraph):
        self._supergraph = supergraph
        self._alias_by_node_id: dict[UUID, str] = {}
        self._node_by_alias: dict[str, Node | NodeReference] = {}
        self._node_by_id: dict[UUID, Node | NodeReference] = {}

    def __str__(self) -> str:
        return ", ".join(self._node_by_alias)

    def __repr__(self) -> str:
        return f"<Aliasing {self}>"

    def clone(self) -> "Aliasing":
        """Clone the current aliasing registry."""
        aliasing = Aliasing(self._supergraph)
        aliasing._alias_by_node_id = self._alias_by_node_id.copy()
        aliasing._node_by_alias = self._node_by_alias.copy()
        aliasing._node_by_id = self._node_by_id.copy()
        return aliasing

    def add(self, obj: Node | NodeReference, alias: str | None = None) -> str:
        """Adds the given nodes to the context of this renderer."""
        # bail if already assigned
        if obj.id in self._alias_by_node_id:
            if alias is not None:
                # ensure alias is set if explicitly given
                self._node_by_alias[alias] = obj
            return self._alias_by_node_id[obj.id]

        # try to resolve node references
        if isinstance(obj, NodeReference):
            if (resolved := self._supergraph.get(obj.id)) is not None:
                obj = resolved

        # make new unique alias if needed
        if alias is None:
            alias = obj.metatype.camel_name if isinstance(obj, Node) else obj.node_type.camel_name
            if alias in self._node_by_alias:
                # bump digit at end to make alias unique
                count = regex.search(r"\d+$", alias)
                if count is None:
                    alias = f"{alias}1"
                    count = 1
                else:
                    count = int(count.group())
                while alias in self._node_by_alias:
                    count += 1
                    alias = regex.sub(r"\d+$", str(count), alias)

        self._alias_by_node_id[obj.id] = alias
        self._node_by_alias[alias] = obj
        self._node_by_id[obj.id] = obj
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

    def __contains__(self, node: Node | NodeReference | UUID) -> bool:
        return self.get(node) is not None

    def resolve(self, name: str) -> Node | NodeReference | None:
        """Resolves the given name to a node. Also attempts to interpret name as an id."""
        node = self._node_by_alias.get(name)
        if node is None:
            # resolve by id
            try:
                name_as_uuid = UUID(name)
                node = self._node_by_id.get(name_as_uuid)
            except ValueError:
                pass
        # try to auto-resolve node references
        if isinstance(node, NodeReference):
            if (resolved := self._supergraph.get(node.id)) is not None:
                node = resolved
        return node

    @staticmethod
    def new(supergraph: Supergraph, aliases: Mapping[str, Node | NodeReference]) -> "Aliasing":
        """Create a new Aliasing registry from a supergraph and a mapping of aliases."""
        aliasing = Aliasing(supergraph)
        for name, node in aliases.items():
            aliasing.add(node, name)
        return aliasing


ACTIVE_ALIASING: contextvars.ContextVar[Aliasing | None] = contextvars.ContextVar("active_aliasing")


def get_active_aliasing() -> Aliasing | None:
    return ACTIVE_ALIASING.get(None)


class Renderer:
    """A renderer to render related objects into code(ish)."""

    def __init__(self, options: RenderOptions):
        self.options = options
        self.aliasing = options.aliasing

    def __str__(self) -> str:
        return f"aliases={len(self.aliasing._node_by_alias)}"

    def __repr__(self) -> str:
        return f"<Renderer {self}>"

    def render_node_ref(self, node: Node | NodeReference) -> str:
        """Renders a python-valid reference to the given node in this context."""
        alias = self.aliasing.get_or_add(node)
        return alias

    def render_property_ref(self, prop: Property | PropertyReference) -> str:
        if isinstance(prop, PropertyReference):
            prop = prop.resolve_or_error()
        return f'{prop.component.__name__}.get_property("{prop.name}")'

    def render_kwargs(self, **kwargs: Any) -> str:
        """Renders kwargs into a string."""
        return ", ".join(f"{k}={v}" for k, v in kwargs.items())

    def render_args(self, *args: Any) -> str:
        """Renders args into a string."""
        return ", ".join(a for a in args if a is not None)

    def render_builtin_object(
        self, obj: BuiltinObjectBase, options: RenderOptions | None = None
    ) -> str:
        """Renders the given object into an expression (incl. inlined children for node)."""
        raise NotImplementedError

    def render_expression(
        self,
        value: BuiltinObjectBase | Property,
        as_ref: bool = False,
        format: bool = False,
    ) -> str:
        """Renders a value into an expression."""
        if isinstance(value, BuiltinObjectBase):
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

    def render_statement(
        self,
        *nodes: Node,
        append: bool = True,
        format: bool = False,
        options: RenderOptions | None = None,
    ) -> str:
        """Renders the given objects to a Python block that defines those objects."""
        # render
        rendered_objs: list[str] = []
        current_children: list[str] = []
        for i, node in enumerate(nodes):
            rendered = self.render_builtin_object(node, options)
            node_alias = self.aliasing.get_or_add(node)
            assert node_alias is not None, f"no alias for {node!r}"
            rendered_objs.append(f"{node_alias} = {rendered}")
            if append and (parent := node.parent) is not None:
                parent_key = self.aliasing.get_or_add(parent)
                next_parent_key = (
                    self.aliasing.get_or_add(nodes[i + 1]) if i < len(nodes) - 1 else None
                )
                if parent_key is not None:
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
    obj: BuiltinObjectBase, *, include_defaults: bool = False, options: RenderOptions
) -> dict[Property, Any]:
    """Gets the 'content' values for a BuiltinObject."""
    cls = type(obj)
    metatype = getattr(obj, "metatype", None)
    assert metatype, f"no metatype for {obj!r}"
    kwargs: dict[Property, Any] = {}
    # properties
    if options.include_properties is not None:
        include_properties = options.include_properties.get(metatype, None)
    else:
        include_properties = None
    if options.exclude_properties is not None:
        exclude_properties = options.exclude_properties.get(metatype, None)
    else:
        exclude_properties = None
    for prop in cls.__properties__.values():
        if exclude_properties is not None and prop in exclude_properties:
            continue  # exclude
        elif include_properties is not None and prop not in include_properties:
            pass  # include
        elif prop.id is None or prop.runtime_prop:
            continue  # ignore internal properties
        prop_value = getattr(obj, prop.name)
        if (
            (prop_value is None and prop.default is None)
            or (isinstance(prop_value, Collection) and len(prop_value) == 0)
            or (prop_value is prop.default and not include_defaults)
        ):
            continue
        kwargs[prop] = prop_value
    return kwargs


def _render_builtin_object_kwargs(
    renderer: Renderer, obj: BuiltinObjectBase, kwargs: dict[Property, Any]
) -> dict[str, str]:
    raise NotImplementedError


#
# Base renderers
#


_renderers: dict[NodeType | StructType, "BuiltinObjectRenderer"] = {}


def _renderer(object_type: NodeType | StructType):
    """Decorator to register a Rewriter for a specific ObjectType."""

    def decorator(cls):
        if object_type in _renderers:
            raise RuntimeError(f"rewriter for {object_type!r} already registered")
        _renderers[object_type] = cls()

    return decorator


class BuiltinObjectRenderer[T: BuiltinObjectBase]:
    """The base renderer for a BuiltinObject."""

    def render(self, renderer: "Renderer", obj: T, options: RenderOptions) -> str:
        """Render the given object to a Python expression (string)."""
        kwargs = _deconstruct_builtin_object(obj, options=options)
        rendered_kwargs = _render_builtin_object_kwargs(renderer, obj, kwargs)
        return f"{obj.__class__.__name__}({renderer.render_kwargs(**rendered_kwargs)})"


#
# Node renderers
#


class NodeRenderer[T: Node](BuiltinObjectRenderer[T]):
    """The base renderer for a Node."""

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
    def render(self, renderer: Renderer, obj: T, options: RenderOptions) -> str:
        kwargs = _deconstruct_builtin_object(obj, options=options)
        rendered_kwargs = _render_builtin_object_kwargs(renderer, obj, kwargs)
        return self._render_constructor(renderer, obj, kwargs, rendered_kwargs)


NODE_RENDERER = NodeRenderer[Node]()


#
# Struct renderers
#


def render_expression(
    value: BuiltinObjectBase | Property, options: RenderOptions, as_ref: bool = False
) -> str:
    """Render the given object to a python expression."""
    renderer = Renderer(options)
    rendered = renderer.render_expression(value, as_ref=as_ref)
    if options.format:
        rendered = format_code(rendered, line_length=options.line_length)
    return rendered.strip()


def render_expressions(
    expressions: Mapping[str, BuiltinObjectBase | Property],
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
def render(*objs: BuiltinObjectBase, options: RenderOptions) -> str: ...
@overload
def render(*objs: Property, options: RenderOptions) -> str: ...
def render(*objs: BuiltinObjectBase | Property, options: RenderOptions) -> str:
    """Renders the given object to either an expression (for values) or statement (for nodes)."""
    if any(isinstance(obj, Node) for obj in objs):
        return render_statement(*cast(list[Node], objs), options=options)
    else:
        # render into tuple of expressions
        value_exprs = [render_expression(obj, options) for obj in objs]
        return ", ".join(value_exprs)
