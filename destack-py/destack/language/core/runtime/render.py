import contextvars
from collections.abc import Mapping
from typing import (
    TYPE_CHECKING,
    assert_never,
    cast,
)

from destack.utils.log import get_logger
from destack.utils.telemetry import get_tracer
from destack.utils.uuid import UUID

from ..builtin import Node, NodeReference
from .graph import Graph

if TYPE_CHECKING:
    pass

logger = get_logger(__name__)
tracer = get_tracer(__name__)


class Aliasing:
    """Registry of aliases for Nodes."""

    def __init__(self, graph: Graph):
        self._graph = graph
        self._alias_by_node_id: dict[UUID, str] = {}
        self._node_by_alias: dict[str, Node | NodeReference] = {}
        self._node_by_id: dict[UUID, Node | NodeReference] = {}

    def __str__(self) -> str:
        return ", ".join(self._node_by_alias)

    def __repr__(self) -> str:
        return f"<Aliasing {self}>"

    def clone(self) -> "Aliasing":
        """Clone the current aliasing registry."""
        aliasing = Aliasing(self._graph)
        aliasing._alias_by_node_id = self._alias_by_node_id.copy()
        aliasing._node_by_alias = self._node_by_alias.copy()
        aliasing._node_by_id = self._node_by_id.copy()
        return aliasing

    def add(self, obj: Node | NodeReference, alias: str | None = None) -> str:
        """Adds the given nodes to the context of this renderer."""
        raise NotImplementedError

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
            if (
                resolved := self._graph.get(
                    id=node.id,
                    space_id=node.space_id,
                    branch_id=node.branch_id,
                    snapshot_id=node.snapshot_id,
                )
            ) is not None:
                node = resolved
        return node

    @staticmethod
    def new(graph: Graph, aliases: Mapping[str, Node | NodeReference]) -> "Aliasing":
        """Create a new Aliasing registry from a graph and a mapping of aliases."""
        aliasing = Aliasing(graph)
        for name, node in aliases.items():
            aliasing.add(node, name)
        return aliasing


ACTIVE_ALIASING: contextvars.ContextVar[Aliasing | None] = contextvars.ContextVar("active_aliasing")


def get_active_aliasing() -> Aliasing | None:
    return ACTIVE_ALIASING.get(None)
