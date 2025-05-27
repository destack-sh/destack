import abc
from collections import deque
from typing import (
    TYPE_CHECKING,
    Collection,
    Optional,
)

import structlog
from fastuuid import UUID
from opentelemetry import trace

from .const import EMPTY_LIST, NodeType

if TYPE_CHECKING:
    from bench.language import Node

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


class GraphError(ValueError):
    pass


class GraphConsistencyError(GraphError):
    pass


def attach_node(
    node: "Node", parent: "Node | None", graph: "Graph", move: bool = False, create: bool = True
):
    raise NotImplementedError


class Graph(abc.ABC):
    __slots__ = ("nodes_by_id", "nodes_by_parent_id")

    def __init__(self):
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
        """Add a node to the graph (error if node already exists, *no* descendants)"""
        raise NotImplementedError

    def update(self, node: "Node"):
        """Updates an existing node in this graph (must exist)"""
        raise NotImplementedError

    def remove(self, node: "Node"):
        """Remove a node from the graph (incl. all descendants)"""
        raise NotImplementedError

    def get_descendants(
        self,
        node: "Node",
        node_type: NodeType | None = None,
        recursive: bool = False,
    ) -> list["Node"]:
        """Collects all descendants as filtered in BFS order"""
        if node.id not in self.nodes_by_parent_id:
            return EMPTY_LIST
        if not recursive:
            if node_type is not None:
                return self.nodes_by_parent_id[node.id].get(node_type, [])
            else:
                all_children: list[Node] = []
                for children in self.nodes_by_parent_id[node.id].values():
                    all_children.extend(children)
                return all_children
        else:
            descendants: list[Node] = []
            if node_type:
                queue = deque(self.nodes_by_parent_id[node.id].get(node_type, []))
            else:
                queue = deque()
                for children in self.nodes_by_parent_id[node.id].values():
                    queue.extend(children)
            while queue:
                cur = queue.popleft()
                descendants.append(cur)
                if node_type:
                    queue.extend(self.nodes_by_parent_id.get(cur.id, {}).get(node_type, ()))
                else:
                    for children in self.nodes_by_parent_id.get(cur.id, {}).values():
                        queue.extend(children)
            return descendants

    def get_ancestors(self, node: "Node") -> list["Node"]:
        """Collects all ancestors up"""
        ancestors: list[Node] = []
        cur = node
        while (parent_id := cur.parent_id) is not None:
            cur = self.nodes_by_id[parent_id]
            ancestors.append(cur)
        return ancestors

    def __bool__(self):
        return True  # not empty

    def extend(self, nodes: Collection["Node"]):
        """Adds all nodes to the graph"""
        for node in nodes:
            self.add(node)


class Supergraph:
    """ """

    __slots__ = ("graphs", "name", "nodes_by_id")

    def __init__(self, name: str):
        self.name = name
        self.nodes_by_id: dict[UUID, Node] = {}
        self.graphs: list[Graph] = []

    def __str__(self):
        return f"{len(self.graphs)} graphs"

    def __repr__(self):
        return f"<Supergraph {self}>"

    def add_graph(self, graph: Graph):
        """Add a graph to this supergraph."""
        raise NotImplementedError

    def remove_graph(self, graph: Graph):
        """Remove a graph from this supergraph."""
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
