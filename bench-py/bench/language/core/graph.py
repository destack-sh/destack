from typing import (
    TYPE_CHECKING,
    Collection,
    Optional,
    Sequence,
)

import structlog
from fastuuid import UUID
from opentelemetry import trace

from .const import UNSET, NodeType

if TYPE_CHECKING:
    from bench.language import Node, QueryConnection, Session

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


class Graph:
    """
    A Graph is a collection of Nodes in a Session.
    Graphs may be tied to the result of a Query (via QueryConnection) or just free-floating.
    """

    __slots__ = ("connection", "nodes_by_id", "nodes_by_parent_id", "supergraph")

    def __init__(self, supergraph: "Supergraph", connection: "QueryConnection | None"):
        self.supergraph = supergraph
        self.connection = connection
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

    def has(self, id: UUID) -> bool:
        """Check if a Node exists in this Graph."""
        return id in self.nodes_by_id

    def clear(self):
        """Clear the Graph."""
        # supergraph
        for node in self.nodes:
            if self.supergraph._cached_nodes_by_id.get(node.id) is node:
                self.supergraph._cached_nodes_by_id.pop(node.id)
        # nodes
        self.nodes_by_id.clear()
        self.nodes_by_parent_id.clear()

    def add(self, node: "Node"):
        """Add a new Node to the graph (must not exist, excluding descendants)."""
        if (existing := self.nodes_by_id.get(node.id)) is not None:
            raise ValueError(f"node {node!r} already in {self!r}: {existing!r}")
        # node
        self.nodes_by_id[node.id] = node
        # parent
        if (parent_ptr := node.parent_ptr) is not None:
            if parent_ptr.id not in self.nodes_by_parent_id:
                self.nodes_by_parent_id[parent_ptr.id] = {}
            if parent_ptr.node_type not in self.nodes_by_parent_id[parent_ptr.id]:
                self.nodes_by_parent_id[parent_ptr.id][parent_ptr.node_type] = []
            self.nodes_by_parent_id[parent_ptr.id][parent_ptr.node_type].append(node)
        # supergraph
        if (
            cached := self.supergraph._cached_nodes_by_id.get(node.id)
        ) is None or cached is _MISSING:
            self.supergraph._cached_nodes_by_id[node.id] = node

    def remove(self, node: "Node"):
        """Remove a Node from the graph (must exist, excluding descendants)."""
        # supergraph
        if self.supergraph._cached_nodes_by_id.get(node.id) is node:
            self.supergraph._cached_nodes_by_id.pop(node.id)
        # parent
        if (parent_ptr := node.parent_ptr) is not None:
            if parent_ptr.id not in self.nodes_by_parent_id:
                self.nodes_by_parent_id[parent_ptr.id] = {}
            if parent_ptr.node_type not in self.nodes_by_parent_id[parent_ptr.id]:
                self.nodes_by_parent_id[parent_ptr.id][parent_ptr.node_type] = []
            self.nodes_by_parent_id[parent_ptr.id][parent_ptr.node_type].remove(node)
            if not self.nodes_by_parent_id[parent_ptr.id][parent_ptr.node_type]:
                self.nodes_by_parent_id[parent_ptr.id].pop(parent_ptr.node_type)
                if not self.nodes_by_parent_id[parent_ptr.id]:
                    self.nodes_by_parent_id.pop(parent_ptr.id)
        # node
        self.nodes_by_id.pop(node.id)

    def get_children[N: Node = Node](
        self,
        node: "Node",
        node_type: NodeType | type[N] | None = None,
    ) -> Sequence[N]:
        """Collects children Nodes. If the Nodes are IsOrdered, their order is preserved."""
        if not self.nodes_by_parent_id:
            return ()
        raise NotImplementedError

    def get_descendants[N: Node = Node](
        self,
        node: "Node",
        node_type: NodeType | type[N] | None = None,
    ) -> Sequence[N]:
        """Collects descendants Nodes. If the Nodes are IsOrdered, their order is preserved."""
        if not self.nodes_by_parent_id:
            return ()
        raise NotImplementedError


_MISSING = object()


class Supergraph:
    """
    A collection of Graphs in a Session.
    Instances of a Node may be in multiple Graphs (but any specific Node is only in one Graph).
    """

    __slots__ = ("_cached_nodes_by_id", "graphs", "session")

    def __init__(self, session: "Session"):
        self.session = session
        self.graphs: list[Graph] = []
        self._cached_nodes_by_id: dict[UUID, Node | object] = {}  # cache

    def __str__(self):
        return f"{len(self.graphs)} graphs"

    def __repr__(self):
        return f"<Supergraph {self}>"

    def add_graph(self, graph: Graph):
        """Add a Graph to this Supergraph."""
        self.graphs.append(graph)
        for node in graph.nodes:
            if (cached := self._cached_nodes_by_id.get(node.id)) is None or cached is _MISSING:
                self._cached_nodes_by_id[node.id] = node

    def remove_graph(self, graph: Graph):
        """Remove a Graph from this Supergraph."""
        self.graphs.remove(graph)

    def get(self, node_id: "UUID") -> Optional["Node"]:
        """Get a node by ID."""
        cached = self._cached_nodes_by_id.get(node_id, UNSET)
        if cached is UNSET:
            # look in all graphs
            for graph in self.graphs:
                if (node := graph.get(node_id)) is not None:
                    self._cached_nodes_by_id[node_id] = node
                    return node
            else:
                self._cached_nodes_by_id[node_id] = _MISSING
                return None
        elif cached is _MISSING:
            return None
        else:
            return cached  # type: ignore (must be Node)

    def get_or_error(self, node_id: "UUID") -> "Node":
        """Get a node by ID (error if not found)."""
        node = self.get(node_id)
        if node is None:
            raise KeyError(f"node {node_id!r} not found in {self!r}")
        return node
