import base64
import contextvars
import dataclasses
import json
from dataclasses import dataclass
from datetime import date, datetime, time, timedelta
from typing import (
    TYPE_CHECKING,
    Any,
    Collection,
    Mapping,
    assert_never,
    cast,
    overload,
    override,
)

import regex
import structlog
from fastuuid import UUID
from opentelemetry import trace

from bench.language.registry import ENUM_CLASS_BY_TYPE
from bench.utils.code import format_code
from bench.utils.time import timedelta_to_isoformat

from .code import Code
from .const import (
    NODE_TYPES,
    EnumType,
    NodeType,
    PrimitiveType,
    StructType,
)
from .graph import Supergraph
from .node import Node
from .object import BuiltinObjectBase
from .property import EdgeType, Property
from .struct import NodeReference, PropertyReference
from .text import Text, TextLine, text_line_to_markdown, text_to_markdown
from .trait import IsInPackage
from .type import TypeBase, TypeCardinality

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
        from bench.runtime.code import PYTHON_KEYWORDS, STATIC_CODE_GLOBALS

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
            alias = obj.metatype.bench_name if isinstance(obj, Node) else obj.node_type.bench_name
            if (
                alias in self._node_by_alias
                or alias in STATIC_CODE_GLOBALS
                or alias in PYTHON_KEYWORDS
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
    return ACTIVE_ALIASING.get()


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

    def render_value_scalar(self, value: "ScalarValue", typ: "TypeBase") -> str:
        """Renders single scalar value into an expression."""
        if typ.cardinality == TypeCardinality.PRIMITIVE:
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
                return repr(value)  # auto-escape
            elif typ.primitive_type == PrimitiveType.UUID:
                return f"UUID({value!r})"
            else:
                return str(value)
        elif typ.cardinality == TypeCardinality.NODE:
            assert isinstance(
                value, (Node, NodeReference)
            ), f"{value!r} is not a node or node reference, expected {typ!r}"
            return self.render_node_ref(value)
        elif typ.cardinality == TypeCardinality.ENUM:
            enum_cls = ENUM_CLASS_BY_TYPE[cast(EnumType, typ.enum_type)]
            value = enum_cls(cast(int, value))
            return f"{enum_cls.__name__}.{value.name}"
        elif typ.cardinality == TypeCardinality.STRUCT:
            if isinstance(value, Property):
                return f"{value.component.__name__}.get_property({value.name!r})"
            else:
                return self.render_builtin_object(cast(Struct, value))
        else:
            raise RuntimeError(f"unexpected type {typ!r}")

    def render_value(self, value: "SomeValue | None", typ: "TypeBase") -> str:
        """Renders a value into an expression."""
        if value is None:
            return "None"
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

    def render_builtin_object(
        self, obj: BuiltinObjectBase, options: RenderOptions | None = None
    ) -> str:
        """Renders the given object into an expression (incl. inlined children for node)."""
        renderer = _get_renderer(obj.metatype)
        return renderer.render(self, obj, options if options is not None else self.options)

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
    kwargs: dict[Property, Any] = {}
    # properties
    if options.include_properties is not None:
        include_properties = options.include_properties.get(cls.metatype, None)
    else:
        include_properties = None
    if options.exclude_properties is not None:
        exclude_properties = options.exclude_properties.get(cls.metatype, None)
    else:
        exclude_properties = None
    for prop in cls.__properties__.values():
        if exclude_properties is not None and prop in exclude_properties:
            continue  # exclude
        elif include_properties is not None and prop not in include_properties:
            pass  # include
        elif prop.id is None or prop.runtime_prop or prop.edge_type == EdgeType.NODE_ANCESTOR:
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
    rendered_kwargs: dict[str, str] = {}
    for prop, value in kwargs.items():
        rendered_kwargs[prop.name] = renderer.render_value(value, prop.type)
    return rendered_kwargs


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


def _get_renderer(object_type: NodeType | StructType) -> "BuiltinObjectRenderer":
    renderer = _renderers.get(object_type)
    if renderer is None:
        if is_node_type(object_type):
            node_type = NodeType(object_type)
            if node_type in PACKAGE_NODE_TYPES:  # noqa: SIM108
                renderer = PACKAGE_NODE_RENDERER
            else:
                renderer = NODE_RENDERER
        else:
            renderer = BUILTIN_OBJECT_RENDERER
    return renderer


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


class IsInPackageRenderer[T: IsInPackage](NodeRenderer[T]):
    """The base renderer for a IsInPackage."""

    @override
    def render(self, renderer: Renderer, obj: T, options: RenderOptions) -> str:
        kwargs = _deconstruct_builtin_object(obj, options=options)
        rendered_kwargs = _render_builtin_object_kwargs(renderer, obj, kwargs)
        return self._render_constructor(renderer, obj, kwargs, rendered_kwargs)


NODE_RENDERER = NodeRenderer[Node]()
PACKAGE_NODE_RENDERER = IsInPackageRenderer[IsInPackage]()
BUILTIN_OBJECT_RENDERER = BuiltinObjectRenderer[BuiltinObjectBase]()


#
# Struct renderers
#


@_renderer(StructType.TEXT_LINE)
class TextLineRenderer(BuiltinObjectRenderer[TextLine]):
    @override
    def render(self, renderer: "Renderer", obj: TextLine, options: RenderOptions) -> str:
        return f"text_line({text_line_to_markdown(obj, renderer.aliasing)!r})"


@_renderer(StructType.TEXT)
class TextRenderer(BuiltinObjectRenderer[Text]):
    @override
    def render(self, renderer: "Renderer", obj: Text, options: RenderOptions) -> str:
        rendered_string = text_to_markdown(obj, renderer.aliasing)
        escaped_string = json.dumps(rendered_string)[1:-1]
        escaped_string = escaped_string.replace('"""', '\\"\\"\\"')
        if "\n" in rendered_string:
            return f'text("""\\\n{escaped_string}\n""")'
        else:
            return f'text("""\\\n{escaped_string}\n""")'


@_renderer(StructType.CODE)
class CodeRenderer(BuiltinObjectRenderer[Code]):
    @override
    def render(self, renderer: "Renderer", obj: Code, options: RenderOptions) -> str:
        rendered_string = obj.to_string()
        if "\n" in rendered_string:
            return f'code("""\\\n{rendered_string}\n""")'
        else:
            return f"code({rendered_string!r})"


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
