import abc
from collections import deque
from typing import (
    TYPE_CHECKING,
    Any,
    ClassVar,
    Collection,
    Iterable,
    Optional,
    cast,
    override,
)
from uuid import UUID

import structlog
from opentelemetry import trace

from bench.language.const import EMPTY_LIST, NodeType, ObjectType
from bench.proto.wire import AnyNodeData, GraphScopeData
from bench.utils.func import IdEnum, bittuple
from bench.utils.string import Casing, to_casing

if TYPE_CHECKING:
    from bench.language import Node, NodeReference
    from bench.proto.wiring import NodeReferenceData

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


class GraphError(ValueError):
    pass


class GraphConsistencyError(GraphError):
    pass


class _NodeGraphBase[K: str | UUID, V: AnyNodeData | Node](abc.ABC):
    __slots__ = (
        "_nodes_by_ck",
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
        self._nodes_by_ck: dict[K, V] = {}  # *most* nodes have a 'ck'
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
        if self.scope.package_id:
            scope_str = f"[bench={self.scope.bench_id}, package={self.scope.package_id}]"
        elif self.scope.bench_id:
            scope_str = f"[bench={self.scope.bench_id}]"
        else:
            scope_str = "[*]"
        return f"{len(self.nodes)} nodes, {node_types_str} {scope_str}"

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

    def get(self, node_id_or_ck: K) -> Optional[V]:
        """Gets a node by id"""
        assert isinstance(node_id_or_ck, self.key_type), f"expected str, got {node_id_or_ck!r}"
        node = self._nodes_by_id.get(node_id_or_ck)
        if node is not None:
            return node
        return self._nodes_by_ck.get(node_id_or_ck)

    def get_or_fail(self, node_id_or_ck: K) -> V:
        """Gets a node by id, raising an error if not found"""
        node = self.get(node_id_or_ck)
        if node is None:
            raise KeyError(f"node {node_id_or_ck!r} not found in {self!r}")
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
        self._nodes_by_ck.clear()
        self._nodes_by_parent.clear()

    @abc.abstractmethod
    def _get_parent_ptr(self, node: V) -> "NodeReferenceData | NodeReference | None": ...

    def _reindex(self):
        """Discoard and rebuild all indexes (useful if node identities change for internally)."""
        nodes = tuple(self._nodes_by_id.values())
        self.clear()
        for node in nodes:
            self.add(node)

    def add(self, node: V):
        """Add a node to the graph (error if node already exists, *no* descendants)"""
        assert isinstance(
            node.id, self.key_type
        ), f"cannot add {node!r} with id {node.id!r} in {self!r}"
        if node.id in self._nodes_by_id:
            existing = self._nodes_by_id[node.id]
            raise GraphConsistencyError(
                f"node {node!r} (id={node.id}) already exists in {self!r}: {existing!r} (id={existing.id})"
            )
        self._nodes_by_id[node.id] = node
        if hasattr(node, "ck"):
            self._nodes_by_ck[getattr(node, "ck")] = node
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
        if hasattr(node, "ck"):
            self._nodes_by_ck[getattr(node, "ck")] = node
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

    def remove(self, node: V):
        """Remove a node from the graph (incl. all descendants)"""
        assert isinstance(
            node.id, self.key_type
        ), f"cannot remove {node!r} with id {node.id!r} in {self!r}"
        existing = self._nodes_by_id.pop(node.id, None)
        if existing is None:
            raise GraphConsistencyError(f"node {node!r} does not exist in {self!r}")
        if hasattr(node, "ck"):
            self._nodes_by_ck.pop(getattr(node, "ck"), None)
        if self._get_parent_ptr(node) is not None:
            self._remove_from_parent(node)
        # descend
        if node.id in self._nodes_by_parent:
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
        assert parent_id in self._nodes_by_parent, f"{node!r} has no parent in {self!r}"
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

    __getitem__ = get_or_fail

    def __contains__(self, item: K):
        return self.get(item) is not None

    def __bool__(self):
        return True  # not empty

    def extend(self, nodes: Collection[V]):
        """Adds all nodes to the graph"""
        for node in nodes:
            self.add(node)


class NodeGraph(_NodeGraphBase[UUID, "Node"]):
    """
    A NodeGraph for Node objects (UUIDs for ids, parent_ptr).
    Nodes must be part of a supergraph.
    """

    key_type = UUID
    value_type = "Node"

    def __init__(
        self,
        scope: GraphScopeData,
        node_types: Collection[NodeType],
        supergraph: "NodeSuperGraph",
        *,
        nodes: Collection["Node"] | None = None,
    ):
        super().__init__(scope, node_types, nodes=nodes)
        self.supergraph = supergraph

    @override
    def _get_parent_ptr(self, node: "Node") -> "NodeReferenceData | NodeReference | None":
        return node.parent_ptr


class NodeDataGraph(_NodeGraphBase[str, AnyNodeData]):
    """
    A NodeGraph for NodeData objects (strings for ids, parent_ptr).
    """

    key_type = str
    value_type = "AnyNodeData"

    @override
    def _get_parent_ptr(self, node: "AnyNodeData") -> "NodeReferenceData | NodeReference | None":
        if node.parent_ptr.metatype != 0:
            return node.parent_ptr
        else:
            return None


class NodeSuperGraph:
    """
    A set of graphs making up the currently available graph in some context (like a session).
    Nodes are resolved against the graphs in the order they were added.
    If the root_ptr is None, this is the 'null' graph.
    """

    __slots__ = ("_base", "_graphs", "_graphs_by_node_type", "_root_ptr")

    def __init__(self, root_ptr: "NodeReference | None", base: "NodeSuperGraph | None" = None):
        self._root_ptr = root_ptr
        self._graphs = ()
        self._graphs_by_node_type: dict[NodeType, tuple[NodeGraph, ...]] = {}
        self._base = base

    def instance(self) -> "NodeSuperGraph":
        """Clone the supergraph, but not the graphs."""
        instance = NodeSuperGraph(self._root_ptr)
        instance._base = self
        instance._graphs = self._graphs
        instance._graphs_by_node_type = {**self._graphs_by_node_type}
        return instance

    def __str__(self):
        return f"{len(self._graphs)} graphs"

    def __repr__(self):
        if self._root_ptr is None:
            return f"<{self.__class__.__name__} <NULL!>>"
        else:
            root = self.get(self._root_ptr)
            root_str = repr(root) if root is not None else f"{self._root_ptr!r}"
            base_str = f", base={self._base!r}" if self._base is not None else ""
            return f"<{self.__class__.__name__} from {root_str} ({self!s}{base_str})>"

    def has(self, other: "NodeSuperGraph"):
        return self is other or (self._base is not None and self._base.has(other))

    @property
    def root(self) -> "Node":
        assert self._root_ptr is not None, f"{self!r} has no root"
        return self.get_or_fail(self._root_ptr)

    def add_graph(self, graph: NodeGraph):
        """Add a graph to this supergraph."""
        assert self is not NULL_SUPERGRAPH, f"cannot add to null graph {self!r}"
        if graph.supergraph is None:
            graph.supergraph = self
        elif graph.supergraph is not self:
            raise RuntimeError(f"{graph!r} is from {graph.supergraph!r}, not {self!r}")
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

    def remove_graph(self, graph: NodeGraph):
        """Remove a graph from this supergraph."""
        assert self is not NULL_SUPERGRAPH, f"cannot add to null graph {self!r}"
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
            # check only graphs that have the node type :)
            graphs = self._graphs_by_node_type.get(ptr.node_type, ())
            for graph in graphs:
                node = graph.get(cast(UUID, ptr.id))
                if node is not None:
                    return node
            return None

    def get_or_fail(self, ptr: "UUID | NodeReference") -> "Node":
        """Get a node by some key (error if not exists)."""
        node = self.get(ptr)
        if node is None:
            raise KeyError(f"node {ptr!r} not found in {self!r}")
        return node

    __getitem__ = get_or_fail

    def __contains__(self, ptr: "UUID | NodeReference") -> bool:
        return self.get(ptr) is not None


NULL_SUPERGRAPH = NodeSuperGraph(root_ptr=None)


def extract_name_id(name: str) -> Optional[int]:
    """Extracts the last (potentially multi-digit) characters as an integer."""
    for i in range(len(name), 0, -1):
        if not name[i - 1].isdigit():
            return None if i == len(name) else int(name[i:])
    return int(name)


def generate_node_name(
    metatype: NodeType, type: Optional[Any], siblings: Collection["Node"]
) -> str:
    """Generates a new name for the given node based on its siblings. :AutoNaming"""
    if metatype == NodeType.BLOCK or metatype == NodeType.VIEW or metatype == NodeType.STEP:
        assert isinstance(type, IdEnum), f"expected type for {metatype!r}, got {type!r}"
        base_name = to_casing(type.name, Casing.CAMEL)
        type_siblings = tuple(n for n in siblings if getattr(n, "type") == type)
    else:
        base_name = to_casing(metatype.name, Casing.CAMEL)
        type_siblings = tuple(n for n in siblings if n.metatype == metatype)

    if len(type_siblings) == 0:
        max_id = 0
    else:
        max_id = max((extract_name_id(getattr(n, "name")) or 0) for n in type_siblings)
    return f"{base_name}{max_id + 1}"


def patch_graph(*, old_graph: NodeGraph, new_graph: NodeGraph) -> None:
    """Patches the existing graph in place from the new graph."""
    for existing_node in list(old_graph.nodes):
        if existing_node.id not in new_graph:
            # node removed: leave as is, remove from existing graph
            old_graph.remove(existing_node)
            continue
        else:
            # node updated: patch in place
            patch_node = new_graph[existing_node.id]
            for prop in existing_node.__wired_properties__.values():
                if prop.is_computed:
                    continue  # ignore computed properties
                prop_value = getattr(existing_node, prop.name)
                existing_node._do_set(prop.name, prop_value, track=False, validate=False)
    for patch_node in list(new_graph.nodes):
        if patch_node.id not in old_graph:
            # node added: add to existing graph
            old_graph.add(patch_node)
