import abc
from collections import deque
from typing import (
    TYPE_CHECKING,
    ClassVar,
    Collection,
    Iterable,
    Optional,
    cast,
    override,
)

import structlog
from fastuuid import UUID
from opentelemetry import trace

from bench.pb2 import AnyNodeData, ScopeData

from .const import EMPTY_LIST, NodeType, bittuple

if TYPE_CHECKING:
    from bench.language import Node, NodeReference
    from bench.proto.wiring import NodeReferenceData

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


class _GraphBase[K: str | UUID, V: AnyNodeData | Node](abc.ABC):
    __slots__ = (
        "_nodes_by_id",
        "_nodes_by_parent",
        "_parent_by_node",
        "node_types",
        "scope",
    )

    key_type: type[K]
    value_type: ClassVar[str]

    def __init__(
        self,
        scope: ScopeData,
        node_types: Collection[NodeType],
        *,
        nodes: Collection[V] | None = None,
    ):
        self.scope = scope
        self.node_types = (
            bittuple(*node_types) if not isinstance(node_types, bittuple) else node_types
        )

        self._nodes_by_id: dict[K, V] = {}
        self._nodes_by_parent: dict[K, dict[NodeType, list[V]]] = {}
        # (nodes may be edited in place, so we remember the last parent id we know manually)
        self._parent_by_node: dict[K, K] = {}

        # add initial nodes
        if isinstance(nodes, Collection):
            for node in nodes:
                self.add(node)
        elif nodes is not None:
            raise ValueError(f"expected nodes, got {nodes!r}")

    def __str__(self):
        return f"{len(self.nodes)} nodes"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}>"

    @property
    def nodes(self) -> Collection[V]:
        """All nodes in the graph"""
        return self._nodes_by_id.values()

    def nodes_of_type[T: Node](self, node_type: type[T]) -> tuple[T, ...]:
        """All nodes of a certain type in the graph"""
        return tuple(n for n in self.nodes if isinstance(n, node_type))

    def __len__(self):
        """Number of nodes in the graph"""
        return len(self._nodes_by_id)

    def add_types(self, *node_types: NodeType):
        """Adds more node types to the graph"""
        self.node_types |= bittuple(*node_types)

    def get(self, node_key: K) -> Optional[V]:
        """Gets a node by id"""
        assert isinstance(node_key, self.key_type), f"expected str, got {node_key!r}"
        return self._nodes_by_id.get(node_key)

    def get_or_error(self, node_key: K) -> V:
        """Gets a node by id, raising an error if not found"""
        node = self.get(node_key)
        if node is None:
            raise KeyError(f"node {node_key!r} not found in {self!r}")
        return node

    def clear(self):
        """Clear the graph"""
        self._nodes_by_id.clear()
        self._nodes_by_parent.clear()

    @abc.abstractmethod
    def _get_parent_ptr(self, node: V) -> "NodeReferenceData | NodeReference | None": ...

    def add(self, node: V):
        """Add a node to the graph (error if node already exists, *no* descendants)"""
        raise NotImplementedError

    def update(self, node: V, _force_update_parent: bool = False):
        """Updates an existing node in this graph (must exist)"""
        raise NotImplementedError

    def remove(self, node: V, recursive: bool = True):
        """Remove a node from the graph (incl. all descendants)"""
        raise NotImplementedError

    def find_roots(self) -> tuple[V, ...]:
        """Finds all root nodes in *this* graph"""
        return tuple(
            node
            for node in self._nodes_by_id.values()
            if node.parent_ptr is None or node.parent_ptr.id not in self._nodes_by_id
        )

    def find_leaves(self) -> tuple[V, ...]:
        """Finds all leaf nodes in *this* graph"""
        return tuple(node for node in self._nodes_by_id.values() if not self.has_descendants(node))

    def has_descendants(self, node: V, child_node_type: NodeType | None = None) -> bool:
        """Checks if a node has descendants of a certain type"""
        if node.id not in self._nodes_by_parent:
            return False
        assert isinstance(node.id, self.key_type), f"expected {self.value_type}, got {node!r}"
        if child_node_type is not None:
            return child_node_type in self._nodes_by_parent[node.id]
        else:
            return True

    def get_descendants(
        self,
        node: V,
        node_type: NodeType | None = None,
        recursive: bool = False,
    ) -> list["V"]:
        """Collects all descendants as filtered in BFS order"""
        assert isinstance(node.id, self.key_type), f"expected {self.value_type}, got {node!r}"
        if node.id not in self._nodes_by_parent:
            return EMPTY_LIST
        if not recursive:
            if node_type is not None:
                return self._nodes_by_parent[node.id].get(node_type, [])
            else:
                all_children: list[V] = []
                for children in self._nodes_by_parent[node.id].values():
                    all_children.extend(children)
                return all_children
        else:
            descendants: list[V] = []
            if node_type:
                queue = deque(self._nodes_by_parent[node.id].get(node_type, []))
            else:
                queue = deque()
                for children in self._nodes_by_parent[node.id].values():
                    queue.extend(children)
            while queue:
                cur = queue.popleft()
                descendants.append(cur)
                if node_type:
                    queue.extend(self._nodes_by_parent.get(cast(K, cur.id), {}).get(node_type, ()))
                else:
                    for children in self._nodes_by_parent.get(cast(K, cur.id), {}).values():
                        queue.extend(children)
            return descendants

    def iter_descendants(
        self,
        node: V,
        node_type: NodeType | None = None,
        recursive: bool = False,
    ) -> Iterable[V]:
        """
        Iterate through filtered descendants in BFS order.
        If recursive, the child node type filter only applies to the first level.
        """
        return iter(self.get_descendants(node, node_type=node_type, recursive=recursive))

    def get_root(self, node: V) -> V:
        """Gets the root node for a given node"""
        root = node
        while self._get_parent_ptr(root) is not None:
            root = self._nodes_by_id[cast(K, root.parent_ptr.id)]  # type: ignore
        return root

    def get_ancestors(self, node: V) -> list[V]:
        """Collects all ancestors up"""
        assert isinstance(node.id, self.key_type), f"expected {self.value_type}, got {node!r}"
        ancestors: list[V] = []
        cur = node
        while self._get_parent_ptr(cur) is not None:
            cur = self._nodes_by_id[cast(K, cur.parent_ptr.id)]  # type: ignore
            ancestors.append(cur)
        return ancestors

    def __bool__(self):
        return True  # not empty

    def extend(self, nodes: Collection[V]):
        """Adds all nodes to the graph"""
        for node in nodes:
            self.add(node)


class Graph(_GraphBase[UUID, "Node"]):
    """
    A Graph for Node objects (UUIDs for ids, parent_ptr).
    Nodes must be part of a supergraph.
    """

    key_type = UUID
    value_type = "Node"

    def __init__(
        self,
        scope: ScopeData,
        node_types: Collection[NodeType],
        supergraph: "Supergraph",
        *,
        nodes: Collection["Node"] | None = None,
    ):
        super().__init__(scope, node_types, nodes=nodes)
        self.supergraph = supergraph

    @override
    def _get_parent_ptr(self, node: "Node") -> "NodeReferenceData | NodeReference | None":
        return node.parent_ptr


class GraphData(_GraphBase[str, AnyNodeData]):
    """
    A Graph for NodeData objects (strings for ids, parent_ptr).
    """

    key_type = str
    value_type = "AnyNodeData"

    @override
    def _get_parent_ptr(self, node: "AnyNodeData") -> "NodeReferenceData | NodeReference | None":
        if node.parent_ptr.metatype != 0:
            return node.parent_ptr
        else:
            return None


class Supergraph:
    """
    A set of graphs making up the currently available graph in some context (like a session).
    Nodes are resolved against the graphs in the order they were added.
    If the root_ptr is None, this is the 'null' graph.
    NOTE :Performance: we should probably just index directly by id in supergraph? (also in bench-web)
    """

    __slots__ = ("_base", "_graphs", "_graphs_by_node_type", "_root_ptr", "name")

    def __init__(
        self, name: str, root_ptr: "NodeReference | None", base: "Supergraph | None" = None
    ):
        self.name = name
        self._root_ptr = root_ptr
        self._graphs = ()
        self._graphs_by_node_type: dict[NodeType, tuple[Graph, ...]] = {}
        self._base = base

    def __str__(self):
        return f"{len(self._graphs)} graphs"

    def __repr__(self):
        if self._root_ptr is None:
            return f"<{self.__class__.__name__} <detached>>"
        else:
            root = self.get(self._root_ptr)
            root_str = repr(root) if root is not None else f"{self._root_ptr!r}"
            base_str = f", base={self._base!r}" if self._base is not None else ""
            return f"<{self.__class__.__name__} {root_str} ({self!s}{base_str})>"

    def has(self, other: "Supergraph"):
        return self is other or (self._base is not None and self._base.has(other))

    def add_graph(self, graph: Graph):
        """Add a graph to this supergraph."""
        raise NotImplementedError

    def remove_graph(self, graph: Graph):
        """Remove a graph from this supergraph."""
        raise NotImplementedError

    def get(self, ptr: "UUID | NodeReference") -> Optional["Node"]:
        """Get a node by some key."""
        raise NotImplementedError


class NullSuperGraph(Supergraph):
    """A null supergraph."""

    def add_graph(self, graph: Graph):
        raise RuntimeError("cannot add graph to null supergraph")

    def remove_graph(self, graph: Graph):
        raise RuntimeError("cannot remove graph from null supergraph")
