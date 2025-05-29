from typing import (
    TYPE_CHECKING,
    Collection,
    Optional,
)

import structlog
from fastuuid import UUID
from opentelemetry import trace

from .const import NodeType

if TYPE_CHECKING:
    from bench.language import Node, Session

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


class GraphError(ValueError):
    pass


class GraphConsistencyError(GraphError):
    pass


class Graph:
    __slots__ = ("nodes_by_id", "nodes_by_parent_id", "supergraph")

    def __init__(self, supergraph: "Supergraph"):
        self.supergraph = supergraph
        self.nodes_by_id: dict[UUID, Node] = {}
        self.nodes_by_parent_id: dict[UUID, dict[NodeType, list[Node]]] = {}

    def __str__(self):
        return f"{len(self.nodes)} nodes"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}>"

    @property
    def nodes(self) -> Collection["Node"]:
        """All nodes in the graph"""
        return self.nodes_by_id.values()

    def __len__(self):
        """Number of nodes in the graph"""
        return len(self.nodes_by_id)

    def get(self, id: UUID) -> Optional["Node"]:
        """Gets a node by id"""
        return self.nodes_by_id.get(id)

    def get_or_error(self, id: UUID) -> "Node":
        """Gets a node by id, raising an error if not found"""
        node = self.get(id)
        if node is None:
            raise KeyError(f"node {id!r} not found in {self!r}")
        return node

    def clear(self):
        """Clear the graph"""
        self.nodes_by_id.clear()
        self.nodes_by_parent_id.clear()

    def add(self, node: "Node"):
        """Add a new Node to the graph (must not exist, excluding descendants)."""
        raise NotImplementedError

    def update(self, node: "Node"):
        """Updates an existing Node (must exist)."""
        raise NotImplementedError

    def remove(self, node: "Node"):
        """Remove a Node from the graph (must exist, excluding descendants)."""
        raise NotImplementedError

    def get_descendants(
        self,
        node: "Node",
        node_type: NodeType | None = None,
        recursive: bool = False,
    ) -> list["Node"]:
        """Collects descendant Nodes in BFS order"""
        raise NotImplementedError

    def extend(self, nodes: Collection["Node"]):
        """Adds all nodes to the graph"""
        for node in nodes:
            self.add(node)


class Supergraph:
    """ """

    __slots__ = ("graphs", "nodes_by_id", "session")

    def __init__(self, session: "Session"):
        self.session = session
        self.nodes_by_id: dict[UUID, Node] = {}
        self.graphs: list[Graph] = []

    def __str__(self):
        return f"{len(self.graphs)} graphs"

    def __repr__(self):
        return f"<Supergraph {self}>"

    def add_graph(self, graph: Graph):
        """Add a Graph to this Supergraph."""
        raise NotImplementedError

    def remove_graph(self, graph: Graph):
        """Remove a Graph from this Supergraph."""
        raise NotImplementedError

    def get(self, node_id: "UUID") -> Optional["Node"]:
        """Get a node by some key."""
        return self.nodes_by_id.get(node_id)

    def get_or_error(self, node_id: "UUID") -> "Node":
        """Get a node by some key, raising an error if not found."""
        node = self.get(node_id)
        if node is None:
            raise KeyError(f"node {node_id!r} not found in {self!r}")
        return node
