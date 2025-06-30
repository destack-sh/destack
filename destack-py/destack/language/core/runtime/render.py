import contextvars
import dataclasses
from collections.abc import Collection, Mapping
from dataclasses import dataclass
from typing import (
    TYPE_CHECKING,
    Any,
    assert_never,
    cast,
)

import regex
import structlog
from opentelemetry import trace

from destack.utils.uuid import UUID

from ..builtin import (
    NODE_TYPES,
    BuiltinObjectBase,
    Node,
    NodeReference,
    NodeType,
    PropertyDeclaration,
    StructType,
)
from .graph import Supergraph

if TYPE_CHECKING:
    pass

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


@dataclass(slots=True)
class RenderOptions:
    aliasing: "Aliasing"
    include_properties: Mapping[NodeType | StructType, Collection[PropertyDeclaration]] | None = (
        None
    )
    exclude_properties: Mapping[NodeType | StructType, Collection[PropertyDeclaration]] | None = (
        None
    )
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
        value: BuiltinObjectBase | PropertyDeclaration,
        as_ref: bool = False,
        format: bool = False,
    ) -> str:
        """Renders a value into an expression."""
        raise NotImplementedError

    def render_statement(
        self,
        *nodes: Node,
        append: bool = True,
        format: bool = False,
        options: RenderOptions | None = None,
    ) -> str:
        """Renders the given objects to a Python block that defines those objects."""
        raise NotImplementedError
