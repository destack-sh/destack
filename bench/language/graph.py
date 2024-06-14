import abc
from collections import deque
from typing import (
    TYPE_CHECKING,
    Any,
    ClassVar,
    Collection,
    Generic,
    Iterable,
    Iterator,
    Mapping,
    Optional,
    TypeVar,
    Union,
    cast,
    overload,
)
from uuid import UUID

import structlog
from more_itertools import first
from opentelemetry import trace

from bench.language.const import EMPTY_LIST, NodeType, ObjectType, ReferenceKind
from bench.language.setup import NODE_CLASS_BY_TYPE, STRUCT_CLASS_BY_TYPE
from bench.language.validation import on_invalid_raise
from bench.proto.wire import AnyNodeData, GraphScope
from bench.utils.casing import Casing, to_casing
from bench.utils.fractional import get_key_bounds, get_order_key, get_order_keys
from bench.utils.func import IdEnum

if TYPE_CHECKING:
    # noinspection PyUnresolvedReferences
    from bench.language import Field, Node, NodeReference, Property, Struct, ValueObject

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


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
        scope: GraphScope,
        node_types: Collection[NodeType],
        *,
        nodes: Collection[V] | None = None,
    ):
        self.scope = scope
        self.node_types = node_types

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

    def clear(self):
        """Clear the graph"""
        self._nodes_by_id.clear()
        self._nodes_by_ck.clear()
        self._nodes_by_parent.clear()

    def add(self, node: V):
        """Add a node to the graph (error if node already exists, *no* descendants)"""
        assert isinstance(
            node.id, self.key_type
        ), f"cannot add {node!r} with id {node.id!r} in {self!r}"
        if node.id in self._nodes_by_id:
            existing = self._nodes_by_id[node.id]
            raise ValueError(
                f"node {node!r} (id={node.id}) already exists in {self!r}: {existing!r} (id={existing.id})"
            )
        self._nodes_by_id[node.id] = node
        if hasattr(node, "ck"):
            self._nodes_by_ck[getattr(node, "ck")] = node
        if node.parent_ptr is not None:
            self._add_to_parent(node)

    def update(self, node: V):
        """Updates an existing node in this graph (must exist)"""
        assert isinstance(
            node.id, self.key_type
        ), f"cannot update {node!r} with id {node.id!r} in {self!r}"
        old = self._nodes_by_id.get(node.id)
        if old is None:
            raise ValueError(f"node {node!r} does not exist in {self!r}")
        self._nodes_by_id[node.id] = node
        if hasattr(node, "ck"):
            self._nodes_by_ck[getattr(node, "ck")] = node
        metatype = cast(ObjectType, node.metatype)

        # update parent if changed
        # (the instance may be edited in place, so we remember the last parent by identity as well)
        old_parent_id = self._parent_by_node.get(
            node.id, old.parent_ptr.id if old.parent_ptr is not None else None
        )
        new_parent_id = node.parent_ptr.id if node.parent_ptr is not None else None
        if old_parent_id != new_parent_id:
            if old_parent_id is not None:
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
                raise ValueError(
                    f"node {node!r} not in {self!r} (should be in {self._nodes_by_parent[old_parent_id][metatype]}, was {old!r})"
                )

    def remove(self, node: V):
        """Remove a node from the graph (incl. all descendants)"""
        assert isinstance(
            node.id, self.key_type
        ), f"cannot remove {node!r} with id {node.id!r} in {self!r}"
        if node.parent_ptr is not None:
            self._remove_from_parent(node)
        self._nodes_by_id.pop(node.id, None)
        if hasattr(node, "ck"):
            self._nodes_by_ck.pop(getattr(node, "ck"), None)
        # descend
        if node.id in self._nodes_by_parent:
            for child_type in tuple(self._nodes_by_parent[node.id]):
                for child in tuple(self._nodes_by_parent[node.id][child_type]):
                    self.remove(child)
                if node.id not in self._nodes_by_parent:
                    break  # may have been removed

    def _add_to_parent(self, node: V):
        """Adds the node to our parent index for that parent/type pair"""
        assert node.parent_ptr is not None, f"{node!r} has no parent for {self!r}"
        parent_id = cast(K, node.parent_ptr.id)
        if parent_id not in self._nodes_by_parent:
            self._nodes_by_parent[parent_id] = {}
        metatype = cast(ObjectType, node.metatype)
        if metatype not in self._nodes_by_parent[parent_id]:
            self._nodes_by_parent[parent_id][metatype] = []
        self._nodes_by_parent[parent_id][metatype].append(node)
        self._parent_by_node[cast(K, node.id)] = parent_id

    def _remove_from_parent(self, node: V):
        """Removes the node from our parent index, cleaning up child containers if empty"""
        assert node.parent_ptr is not None, f"{node!r} has no parent for {self!r}"
        parent_id = self._parent_by_node.get(cast(K, node.id))
        if parent_id is None:
            parent_id = cast(K | None, node.parent_ptr.id)
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

    def has_descendants(self, node: V, child_node_type: NodeType | None = None) -> bool:
        """Checks if a node has descendants of a certain type"""
        if node.id not in self._nodes_by_parent:
            return False
        assert isinstance(node.id, self.key_type), f"expected {self.value_type}, got {node!r}"
        if child_node_type is not None:
            return child_node_type in self._nodes_by_parent[node.id]
        else:
            return True

    def collect_descendants(
        self, node: V, child_node_type: NodeType | None = None, recursive: bool = False
    ) -> list["V"]:
        """Collects all descendants as filtered in BFS order"""
        assert isinstance(node.id, self.key_type), f"expected {self.value_type}, got {node!r}"
        if node.id not in self._nodes_by_parent:
            return EMPTY_LIST
        if not recursive:
            if child_node_type is not None:
                return self._nodes_by_parent[node.id].get(child_node_type, [])
            else:
                all_children: list[V] = []
                for children in self._nodes_by_parent[node.id].values():
                    all_children.extend(children)
                return all_children
        else:
            descendants: list[V] = []
            if child_node_type:
                queue = deque(self._nodes_by_parent[node.id].get(child_node_type, []))
            else:
                queue = deque()
                for children in self._nodes_by_parent[node.id].values():
                    queue.extend(children)
            while queue:
                cur = queue.popleft()
                descendants.append(cur)
                for children in self._nodes_by_parent.get(cast(K, cur.id), {}).values():
                    queue.extend(children)
            return descendants

    def get_root(self, node: V) -> V:
        """Gets the root node for a given node"""
        root = node
        while root.parent_ptr is not None:
            root = self._nodes_by_id[cast(K, root.parent_ptr.id)]
        return root

    def iter_descendants(
        self, node: V, child_node_type: NodeType | None = None, recursive: bool = False
    ) -> Iterable[V]:
        """
        Iterate through filtered descendants in BFS order.
        If recursive, the child node type filter only applies to the first level.
        """
        return iter(self.collect_descendants(node, child_node_type, recursive))

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

    def set(self, nodes: Collection[V]):
        """Replaces all nodes in the graph"""
        self.clear()
        for node in nodes:
            self.add(node)

    def add_graph(self, graph: "_NodeGraphBase[K, V]"):
        """Adds all nodes from another graph"""
        for node in graph.nodes:
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
        scope: GraphScope,
        node_types: Collection[NodeType],
        supergraph: "NodeSuperGraph",
        *,
        nodes: Collection["Node"] | None = None,
    ):
        super().__init__(scope, node_types, nodes=nodes)
        self.supergraph = supergraph


class NodeDataGraph(_NodeGraphBase[str, AnyNodeData]):
    """
    A NodeGraph for NodeData objects (strings for ids, parent_ptr).
    """

    key_type = str
    value_type = "AnyNodeData"


class NodeSuperGraph:
    """A set of graphs making up the currently available graph in some context (like a session)."""

    def __init__(self, root_ptr: "NodeReference", graphs: Collection[NodeGraph] | None = None):
        self._root_ptr = root_ptr
        self._graphs = list(graphs) if graphs is not None else []

    def __str__(self):
        return f"{len(self._graphs)} graphs"

    def __repr__(self):
        root = self.get(self._root_ptr)
        root_str = repr(root) if root is not None else f"{self._root_ptr!r}"
        return f"<{self.__class__.__name__} from {root_str} ({self!s})>"

    @property
    def root(self) -> "Node":
        return self.get_or_fail(self._root_ptr)

    def add_graph(self, graph: NodeGraph):
        if graph.supergraph is None:
            graph.supergraph = self
        assert graph.supergraph is self, f"{graph!r} is from {graph.supergraph!r}, not {self!r}"
        assert graph not in self._graphs, f"{graph!r} already in {self!r}"
        self._graphs.append(graph)

    def remove_graph(self, graph: NodeGraph):
        assert graph in self._graphs, f"{graph!r} not in {self!r}"
        self._graphs.remove(graph)

    def get(self, ptr: "UUID | NodeReference") -> Optional["Node"]:
        if isinstance(ptr, UUID):
            key = ptr
        else:
            key = ptr.id
            assert key is not None, f"expected id for {ptr!r}"
        for graph in self._graphs:
            node = graph.get(key)
            if node is not None:
                return node
        return None

    def get_or_fail(self, ptr: "UUID | NodeReference") -> "Node":
        node = self.get(ptr)
        if node is None:
            raise KeyError(f"node {ptr!r} not found in {self!r}")
        return node

    __getitem__ = get_or_fail


class _NodeDictBase[K, V]:
    """A simple graph-like wrapper for a dict of nodes that has some of the same methods."""

    key_type: type[K]

    def __init__(self, nodes_by_id: Mapping[K, V]):
        self._nodes_by_id = nodes_by_id

    def __str__(self):
        return f"{len(self._nodes_by_id)} nodes"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}>"

    @property
    def nodes(self):
        return self._nodes_by_id.values()

    def __getitem__(self, item: K) -> V:
        assert item in self._nodes_by_id, f"expected {self.key_type}, got {item!r}"
        return self._nodes_by_id[item]

    def __contains__(self, item: K) -> bool:
        assert isinstance(item, self.key_type), f"expected {self.key_type}, got {item!r}"
        return item in self._nodes_by_id

    def get(self, item: K) -> V:
        assert isinstance(item, self.key_type), f"expected {self.key_type}, got {item!r}"
        return self._nodes_by_id[item]


class NodeDict(_NodeDictBase[UUID, "Node"]):
    key_type = UUID


class NodeDataDict(_NodeDictBase[str, AnyNodeData]):
    key_type = str


NodeGraphLike = Union[NodeGraph, NodeDict]
NodeDataGraphLike = Union[NodeDataGraph, NodeDataDict]


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


class NodeList[V: Node](abc.ABC, Collection[V]):
    """
    A list of node descendants of a parent's property.
    This is the primary way of adding, removing and accessing regular node relations.
    """

    __slots__ = ("_parent", "_property")

    def __init__(self, parent: "Node", property: "Property"):
        self._parent = parent
        self._property = property

    def __repr__(self):
        return f"<{self.__class__.__name__} {self._parent.absolute_path}.{self._property.name}: {self}>"

    @property
    def nodes(self) -> tuple[V, ...] | list[V]:
        raise NotImplementedError

    def create(self, **kwargs) -> V:
        """Creates a new node in the list."""
        from bench.language.node import NODE_CLASS_BY_TYPE

        node_metatype = cast(list[NodeType], self._property.reference_nodes)[0]
        node_cls = NODE_CLASS_BY_TYPE[node_metatype]
        # auto-generate name if required and not given :AutoNaming
        if "name" in node_cls.__properties__ and "name" not in kwargs:
            kwargs["name"] = generate_node_name(node_metatype, kwargs.get("type"), self)
        # set new node status to source to prevent activation before it's appended
        node = node_cls(**kwargs)
        node = cast(V, node)
        self.append(node)
        return node

    @abc.abstractmethod
    def append(self, node: V) -> V:
        """Attaches a child node to a parent through a list."""
        raise NotImplementedError

    @abc.abstractmethod
    def extend(self, *nodes: V):
        """Attaches a list of child nodes to a parent. See append."""
        raise NotImplementedError

    @abc.abstractmethod
    def remove(self, node: V):
        """Removes a child node from a parent. See append for reverse."""
        raise NotImplementedError

    @abc.abstractmethod
    def clear(self):
        """Removes all child nodes from a parent. See append for reverse."""
        raise NotImplementedError

    def set(self, nodes: Collection[V]):
        """Replaces all child nodes of a parent."""
        self.clear()
        self.extend(*nodes)

    @abc.abstractmethod
    def get(self, key: UUID | str | int) -> V | None:
        """Gets a node by some key (id/ck, actual name or identifier)."""
        raise NotImplementedError

    def __contains__(self, obj: object | V | str | UUID) -> bool:
        """Checks if a node is in the list."""
        if type(obj) is str or type(obj) is UUID:
            return self.get(obj) is not None
        else:
            return obj in self

    @overload
    def __getitem__(self, item: str) -> Optional[V]: ...
    @overload
    def __getitem__(self, item: UUID) -> Optional[V]: ...
    @overload
    def __getitem__(self, item: int) -> V: ...
    @overload
    def __getitem__(self, item: slice) -> list[V]: ...
    def __getitem__(self, item: Union[str, UUID, int, slice]):
        """Gets a node by index or name."""
        if isinstance(item, (str, UUID)):
            return self.get(item)
        else:
            return self.nodes[item]

    def __getattr__(self, item: str) -> V:
        """Gets a node by name."""
        node = self.get(item)
        if node is None:
            raise AttributeError(f"{self!r} has no node {item!r}")
        return node

    def tolist(self) -> list[V]:
        return list(self)


class GraphNodeList[V: Node](NodeList[V]):
    # TODO :Cleanup :Architecture: use ReadQuery/WriteQuery in NodeList?
    #  (with InMemoryGraphEngine to query)
    __slots__ = ("_child_node_type", "_flags")

    def __init__(self, parent: "Node", property: "Property"):
        super().__init__(parent, property)
        assert (
            property.reference_nodes and len(property.reference_nodes) == 1
        ), f"cannot have many child types: {property!r}"
        self._child_node_type: NodeType = property.reference_nodes[0]

    def __str__(self):
        return str(self.nodes)

    def get(self, key: UUID | str | int) -> V | None:
        if isinstance(key, UUID):
            return cast(V, self._parent._graph.get(key))
        elif isinstance(key, str):
            return first(
                (n for n in self.nodes if n.py_ident == key or getattr(n, "name", None) == key),
                None,
            )
        else:
            return self.nodes[key]

    @property
    def nodes(self) -> tuple[V, ...] | list[V]:
        """Access the computed nodes"""
        descendants = self._parent._graph.collect_descendants(
            node=self._parent, child_node_type=self._child_node_type, recursive=False
        )
        if (
            len(descendants) > 1
            and "order_key" in NODE_CLASS_BY_TYPE[self._child_node_type].__properties__
        ):
            descendants.sort(key=lambda n: n.order_key)  # type: ignore
        return cast(list[V], descendants)

    def append(  # type: ignore
        self, node: V, after: V | None = None, before: V | None = None
    ) -> tuple[V, ...]:
        from bench.language.node import Node

        assert isinstance(node, Node), f"cannot append {node!r} to {self!r}"
        if node.parent is not None:
            raise ValueError(f"cannot attach {node!r} to {self!r}: attached to {node.parent!r}")

        # validate early
        node.parent = self._parent
        if self._parent._session is not None:
            node._validate_self((), invalid=on_invalid_raise)

        # add node (and descendants) to this parent's graph
        if node._graph is not self._parent._graph:
            added = node._graph.collect_descendants(node, recursive=True)
            added = (*added, node)
            self._parent._graph.add_graph(node._graph)
            for n in added:
                n._graph = self._parent._graph
        else:
            added = (node,)
            self._parent._graph.add(node)

        # assign order key to ordered nodes
        if hasattr(node, "order_key") and getattr(node, "order_key") is None:
            ok = get_order_key(*get_key_bounds(self.nodes, after, before))
            setattr(node, "order_key", ok)

        # 'create' node in session if it's attached
        if self._parent._session and self._parent._is_attached:
            self._parent._session.create(*added)
            self._parent._session.track_many(*added)

        return cast(tuple[V, ...], added)

    def extend(self, *nodes: V, after: V | None = None, before: V | None = None) -> None:  # type: ignore
        if not nodes:
            return

        # pre-assign order keys since we don't trigger between appends (meaning last_ok is wrong)
        if hasattr(nodes[0], "order_key") and getattr(nodes[0], "order_key") is None:
            oks = get_order_keys(*get_key_bounds(self.nodes, after, before), n=len(nodes))
            for node, ok in zip(nodes, oks):
                setattr(node, "order_key", ok)

        for node in nodes:
            self.append(node)

    def remove(self, n: V):  # type: ignore
        if self._parent._session is not None:
            self._parent._session.delete(n)
        self._parent._graph.remove(n)
        n.parent = None

    def clear(self):
        if not self.nodes:
            return
        removed = tuple(self.nodes)
        for n in removed:
            self.remove(n)

    def __bool__(self):
        return len(self.nodes) > 0

    def __contains__(self, obj: object) -> bool:
        if isinstance(obj, Node):
            if obj in self.nodes:
                return True
            else:
                if (
                    self._property.reference_nodes is not None
                    and obj.metatype not in self._property.reference_nodes
                ):
                    raise TypeError(f"{self!r} cannot contain {obj!r}")
                return False
        else:
            return False

    def __iter__(self) -> Iterator[V]:
        yield from self.nodes

    def __len__(self) -> int:
        return len(self.nodes)

    def __eq__(self, other: object) -> bool:
        if isinstance(other, GraphNodeList):
            return self.nodes == other.nodes
        elif isinstance(other, list):
            return self.nodes == other
        else:
            return False


ValueParentT = TypeVar("ValueParentT", bound=Union["ValueObject", "Struct", "Node"])
ValueT = TypeVar("ValueT", bound=Union["ValueObject", "Struct", "Property"])
ValueProperty = Union["Property", "Field"]


class ValueList(list, Generic[ValueParentT]):
    """
    A list of Values or Value-like objects (with local identity, so can't be inlined).
    Unlike a NodeList, value lists are actual lists and not computed on access.
    NOTE :Cleanup: shouldn't ValueList be in value.py?
    """

    def __init__(
        self,
        parent: ValueParentT,
        parent_prop: ValueProperty,
        ancestor_prop: Optional["Property"] = None,
        *args,
        **kwargs,
    ):  # type: ignore
        from bench.language.node import Property

        super().__init__(*args, **kwargs)
        self.parent = parent
        self.parent_prop = parent_prop
        if isinstance(parent_prop, Property):
            self.ancestor_prop = parent_prop
            self.parent_key = parent_prop.id_as_str
            self.is_ordered = (
                parent_prop.reference_kind == ReferenceKind.STRUCT_CHILD
                and not STRUCT_CLASS_BY_TYPE[
                    cast(Any, parent_prop.reference_struct)
                ].__is_struct_inlined__
            )
        else:  # Field
            assert ancestor_prop is not None, f"expected ancestor_prop for {parent_prop!r}"
            self.ancestor_prop = ancestor_prop
            self.parent_key = parent_prop.identity_key
            self.is_ordered = True
        self.is_property_reference = (
            isinstance(parent_prop, Property) and parent_prop.is_property_reference
        )

    def append(self, item: ValueT, after: ValueT | None = None, before: ValueT | None = None):  # type: ignore
        if not self.is_property_reference:
            item = item._move_to(self.parent, self.parent_prop)  # type: ignore
        super().append(item)
        if self.is_ordered:
            cast(Union["ValueObject", "Struct"], item).order_key = get_order_key(
                *get_key_bounds(self, after, before)
            )
        self.parent._updated_self((self.ancestor_prop,))
        return item

    def extend(self, items: Collection[ValueT]):  # type: ignore
        super().extend(items)
        if not self.is_property_reference:
            values = cast(list[Union["ValueObject", "Struct"]], items)
            if any(item.parent is not None for item in values):
                values = [e._copy_to(self.parent, self.parent_prop) for e in items]  # type: ignore
            else:
                for item in values:
                    item.parent = self.parent
                    item.parent_key = self.parent_key
            if self.is_ordered:
                order_keys = get_order_keys(*get_key_bounds(self), n=len(items))
                for item, order_key in zip(values, order_keys):
                    item.order_key = order_key
        self.parent._updated_self((self.ancestor_prop,))

    def clear(self):
        super().clear()
        self.parent._updated_self((self.ancestor_prop,))

    @staticmethod
    def _move_list(
        values: Collection[ValueT],
        parent: ValueParentT,
        parent_prop: ValueProperty,
        ancestor_prop: Optional["Property"] = None,
    ):
        """Moves or copies the values in the list to the given parent."""
        from bench.language.node import Property

        parent_key = (
            parent_prop.id_as_str if isinstance(parent_prop, Property) else parent_prop.identity_key
        )
        if any(
            v.parent is not None and (v.parent != parent or v.parent_key != parent_key)
            for v in cast(list[Union["ValueObject", "Struct"]], values)
        ):
            values = [v._copy_to(parent, parent_prop) for v in values]  # type: ignore
        return ValueList(parent, parent_prop, ancestor_prop, values)
