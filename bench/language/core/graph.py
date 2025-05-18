import abc
from collections import deque
from typing import (
    TYPE_CHECKING,
    ClassVar,
    Collection,
    Iterable,
    Optional,
    Self,
    cast,
    override,
)
from uuid import UUID

import structlog
from opentelemetry import trace

from bench.pb2 import AnyNodeData, GraphScopeData

from .const import EMPTY_LIST, NodeType, ObjectType, bittuple

if TYPE_CHECKING:
    from bench.language import Node, NodeReference
    from bench.proto.wiring import NodeReferenceData

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


class GraphError(ValueError):
    pass


class GraphConsistencyError(GraphError):
    pass


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
        scope: GraphScopeData,
        node_types: Collection[NodeType],
        *,
        nodes: Collection[V] | None = None,
    ):
        self.scope = scope
        self.node_types = (
            bittuple(*node_types) if not isinstance(node_types, bittuple) else node_types
        )

        self._nodes_by_id: dict[K, V] = {}
        self._nodes_by_parent: dict[K, dict[ObjectType, list[V]]] = {}
        # (nodes may be edited in place, so we remember the last parent id we know manually)
        self._parent_by_node: dict[K, K] = {}

        # add initial nodes
        if isinstance(nodes, Collection):
            for node in nodes:
                self.add(node)
        elif nodes is not None:
            raise ValueError(f"expected nodes, got {nodes!r}")

    def __str__(self):
        node_types_str = "|".join(nt.bench_name for nt in self.node_types)
        if self.scope.package_ids:
            scope_str = f"[bench={self.scope.bench_id}, packages={', '.join(str(id) for id in self.scope.package_ids)}]"
        elif self.scope.bench_id:
            scope_str = f"[bench={self.scope.bench_id}]"
        else:
            scope_str = "[*]"
        return f"{len(self.nodes)} nodes, {node_types_str} {scope_str}"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}>"

    def copy(self) -> Self:
        """Copy the graph"""
        return self.__class__(self.scope, self.node_types, nodes=self.nodes)

    @property
    def nodes(self) -> Collection[V]:
        """All nodes in the graph"""
        return self._nodes_by_id.values()

    def nodes_of_type[T: Node](self, node_type: type[T]) -> tuple[T, ...]:
        """All nodes of a certain type in the graph"""
        return tuple(n for n in self.nodes if isinstance(n, node_type))

    def nodes_bfs(self, node_type: NodeType | None = None) -> Iterable[V]:
        """Iterate through nodes in BFS order"""
        roots = self.find_roots()
        queue = deque(roots)
        while queue:
            node = queue.popleft()
            yield node
            queue.extend(self.get_descendants(node, node_type=node_type))

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

    def get_by_name(self, name: str, node_type: NodeType | None = None) -> V | None:
        """Gets a node by name"""
        # NOTE :Performance: index node names :NodeNameIndexing
        if node_type is None:
            for node in self._nodes_by_id.values():
                if getattr(node, "name", None) == name:
                    return node
        else:
            for node in self._nodes_by_id.values():
                if node.metatype == node_type and getattr(node, "name", None) == name:
                    return node
        return None

    def clear(self):
        """Clear the graph"""
        self._nodes_by_id.clear()
        self._nodes_by_parent.clear()

    @abc.abstractmethod
    def _get_parent_ptr(self, node: V) -> "NodeReferenceData | NodeReference | None": ...

    def _reindex(self):
        """Discoard and rebuild all indexes (internal use when node identities change)."""
        nodes = tuple(self._nodes_by_id.values())
        self.clear()
        for node in nodes:
            self.add(node)

    def add(self, node: V):
        """Add a node to the graph (error if node already exists, *no* descendants)"""
        assert isinstance(
            node.id, self.key_type
        ), f"cannot add {node!r} with id {node.id!r} in {self!r}"
        assert node.metatype in self.node_types, f"{node!r} does not belong in in {self!r}"
        if node.id in self._nodes_by_id:
            existing = self._nodes_by_id[node.id]
            raise GraphConsistencyError(
                f"node {node!r} (id={node.id}) already exists in {self!r}: {existing!r} (id={existing.id})"
            )
        self._nodes_by_id[node.id] = node
        if self._get_parent_ptr(node) is not None:
            self._add_to_parent(node)

    def update(self, node: V, _force_update_parent: bool = False):
        """Updates an existing node in this graph (must exist)"""
        assert isinstance(
            node.id, self.key_type
        ), f"cannot update {node!r} with id {node.id!r} in {self!r}"
        old = self._nodes_by_id.get(node.id)
        if old is None:
            raise GraphConsistencyError(f"node {node!r} does not exist in {self!r}")
        self._nodes_by_id[node.id] = node
        metatype = cast(ObjectType, node.metatype)

        # update parent if changed
        # (the instance may be edited in place, so we remember the last parent by identity as well)
        old_parent_ptr = self._get_parent_ptr(old)
        old_parent_id = self._parent_by_node.get(
            node.id, old_parent_ptr.id if old_parent_ptr is not None else None
        )
        node_parent_ptr = self._get_parent_ptr(node)
        new_parent_id = node_parent_ptr.id if node_parent_ptr is not None else None
        if old_parent_id != new_parent_id or _force_update_parent:
            if old_parent_id is not None and not _force_update_parent:
                self._remove_from_parent(old)
            if new_parent_id is not None:
                self._add_to_parent(node)
        elif old_parent_id is not None:
            # update in parent list (identity may have changed)
            assert isinstance(old_parent_id, self.key_type), f"bad {old_parent_id!r} for {self!r}"
            for i, child in enumerate(self._nodes_by_parent[old_parent_id][metatype]):
                if child.id == node.id:
                    self._nodes_by_parent[old_parent_id][metatype][i] = node
                    break
            else:
                raise GraphConsistencyError(
                    f"node {node!r} not in {self!r} (should be in {self._nodes_by_parent[old_parent_id][metatype]}, was {old!r})"
                )

    def remove(self, node: V, recursive: bool = True):
        """Remove a node from the graph (incl. all descendants)"""
        assert isinstance(
            node.id, self.key_type
        ), f"cannot remove {node!r} with id {node.id!r} in {self!r}"
        existing = self._nodes_by_id.pop(node.id, None)
        if existing is None:
            raise GraphConsistencyError(f"node {node!r} does not exist in {self!r}")
        if self._get_parent_ptr(node) is not None:
            self._remove_from_parent(node)
        # descend
        if recursive and node.id in self._nodes_by_parent:
            for child_type in tuple(self._nodes_by_parent[node.id]):
                for child in tuple(self._nodes_by_parent[node.id][child_type]):
                    self.remove(child)
                if node.id not in self._nodes_by_parent:
                    break  # may have been removed

    def _add_to_parent(self, node: V):
        """Adds the node to our parent index for that parent/type pair"""
        node_parent_ptr = self._get_parent_ptr(node)
        assert node_parent_ptr is not None, f"{node!r} has no parent for {self!r}"
        parent_id = cast(K, node_parent_ptr.id)
        if parent_id not in self._nodes_by_parent:
            self._nodes_by_parent[parent_id] = {}
        metatype = cast(ObjectType, node.metatype)
        if metatype not in self._nodes_by_parent[parent_id]:
            self._nodes_by_parent[parent_id][metatype] = []
        self._nodes_by_parent[parent_id][metatype].append(node)
        self._parent_by_node[cast(K, node.id)] = parent_id

    def _remove_from_parent(self, node: V):
        """Removes the node from our parent index, cleaning up child containers if empty"""
        node_parent_ptr = self._get_parent_ptr(node)
        assert node_parent_ptr is not None, f"{node!r} has no parent for {self!r}"
        parent_id = self._parent_by_node.get(cast(K, node.id))
        if parent_id is None:
            parent_id = cast(K | None, node_parent_ptr.id)
            assert parent_id, f"{node!r} has no parent for {self!r}"
        else:
            del self._parent_by_node[cast(K, node.id)]
        if parent_id not in self._nodes_by_parent:
            return  # we don't have this parent
        metatype = node.metatype
        assert metatype in self._nodes_by_parent[parent_id], f"{node!r} not in {self!r}"
        # node may be different instance, find by id
        our_node = None
        for n in self._nodes_by_parent[parent_id][metatype]:
            if n.id == node.id:
                our_node = n
                break
        assert our_node is not None, f"node {node!r} not in {self!r}"
        self._nodes_by_parent[parent_id][metatype].remove(our_node)
        if len(self._nodes_by_parent[parent_id][metatype]) == 0:
            self._nodes_by_parent[parent_id].pop(metatype)
        if len(self._nodes_by_parent[parent_id]) == 0:
            self._nodes_by_parent.pop(parent_id)

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

    # utilities

    __getitem__ = get_or_error

    def __contains__(self, item: K):
        return self.get(item) is not None

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
        scope: GraphScopeData,
        node_types: Collection[NodeType],
        supergraph: "Supergraph",
        *,
        nodes: Collection["Node"] | None = None,
    ):
        super().__init__(scope, node_types, nodes=nodes)
        self.supergraph = supergraph

    @override
    def copy(self) -> Self:
        """Copy the graph"""
        return self.__class__(self.scope, self.node_types, self.supergraph, nodes=self.nodes)

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

    def instance(self, name: str) -> "Supergraph":
        """Clone the supergraph, but not the graphs."""
        instance = Supergraph(name, self._root_ptr)
        instance._base = self
        instance._graphs = self._graphs
        instance._graphs_by_node_type = {**self._graphs_by_node_type}
        return instance

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

    @property
    def root(self) -> "Node":
        assert self._root_ptr is not None, f"{self!r} has no root"
        return self.get_or_error(self._root_ptr)

    def add_graph(self, graph: Graph):
        """Add a graph to this supergraph."""
        if graph.supergraph is None:
            graph.supergraph = self
        elif graph.supergraph is not self:
            raise RuntimeError(f"{graph!r} is already in {graph.supergraph!r}, not {self!r}")
        assert graph not in self._graphs, f"{graph!r} already in {self!r}"
        self._graphs = (*self._graphs, graph)
        for node_type in graph.node_types:
            if node_type not in self._graphs_by_node_type:
                self._graphs_by_node_type[node_type] = (graph,)
            else:
                self._graphs_by_node_type[node_type] = (
                    *self._graphs_by_node_type[node_type],
                    graph,
                )

    def remove_graph(self, graph: Graph):
        """Remove a graph from this supergraph."""
        assert graph in self._graphs, f"{graph!r} not in {self!r}"
        self._graphs = tuple(g for g in self._graphs if g is not graph)
        for node_type in graph.node_types:
            self._graphs_by_node_type[node_type] = tuple(
                g for g in self._graphs_by_node_type[node_type] if g is not graph
            )

    def get(self, ptr: "UUID | NodeReference") -> Optional["Node"]:
        """Get a node by some key."""
        if isinstance(ptr, UUID):
            # check all graphs :c
            for graph in self._graphs:
                node = graph.get(ptr)
                if node is not None:
                    return node
            return None
        else:
            # check only graphs that have the node type
            graphs = self._graphs_by_node_type.get(ptr.node_type, ())
            for graph in graphs:
                node = graph.get(ptr.id)
                if node is not None:
                    return node
            return None

    def get_or_error(self, ptr: "UUID | NodeReference") -> "Node":
        """Get a node by some key (error if not exists)."""
        node = self.get(ptr)
        if node is None:
            raise KeyError(f"node {ptr!r} not found in {self!r}")
        return node

    def get_graphs(self, node_type: NodeType) -> tuple[Graph, ...]:
        """Get all graphs that have a certain node type."""
        return self._graphs_by_node_type.get(node_type, ())

    __getitem__ = get_or_error

    def __contains__(self, ptr: "UUID | NodeReference") -> bool:
        return self.get(ptr) is not None


class NullSuperGraph(Supergraph):
    """A null supergraph."""

    def add_graph(self, graph: Graph):
        raise RuntimeError("cannot add graph to null supergraph")

    def remove_graph(self, graph: Graph):
        raise RuntimeError("cannot remove graph from null supergraph")


NULL_SUPERGRAPH = NullSuperGraph(name="<NULL>", root_ptr=None)
