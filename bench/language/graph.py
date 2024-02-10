import abc
from collections import defaultdict, deque
from typing import (
    TYPE_CHECKING,
    Collection,
    Generator,
    Generic,
    Iterable,
    Iterator,
    Optional,
    TypeVar,
    Union,
)
from uuid import UUID

from bench.language.const import NodeStatus, NodeType, NRel
from bench.language.validation import on_invalid_raise
from bench.proto import wire
from bench.proto.wire import AnyNodeData
from bench.utils.fractional import generate_key_between, generate_n_keys_between
from bench.utils.func import nextn

if TYPE_CHECKING:
    from bench.language import Node, Property, ScopeNode

NodeT = TypeVar("NodeT", bound="Node")
NodeDataT = TypeVar("NodeDataT", bound=AnyNodeData)
SomeNodeT = TypeVar("SomeNodeT")
IdT = TypeVar("IdT", bound=Union[UUID, str])


class NodeGraphBase(abc.ABC, Generic[SomeNodeT, IdT]):
    def __str__(self):
        return f"{len(self.nodes)} nodes"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}>"

    @property
    def nodes(self) -> Collection[SomeNodeT]:
        raise NotImplementedError

    def copy(self) -> "NodeGraphBase[SomeNodeT, IdT]":
        raise NotImplementedError

    def get(self, node_id_or_ck: IdT) -> Optional[SomeNodeT]:
        """Gets a node by id or ck"""
        raise NotImplementedError

    def clear(self):
        """Clear the graph"""
        raise NotImplementedError

    def add(self, node: "SomeNodeT"):
        """Add a node to the graph (error if node already exists, *no* descendants)"""
        raise NotImplementedError

    def update(self, node: "SomeNodeT"):
        """Updates an existing node in this graph (must exist)"""
        raise NotImplementedError

    def remove(self, node: "SomeNodeT"):
        """Remove a node from the graph (incl. all descendants)"""
        raise NotImplementedError

    def find_roots(self) -> tuple[SomeNodeT, ...]:
        raise NotImplementedError

    def find_root(self) -> Optional[SomeNodeT]:
        roots = self.find_roots()
        if len(roots) > 1:
            raise ValueError(f"expected 0 or 1 root nodes, got {roots}")
        return roots[0] if roots else None

    def collect_descendants(
        self, node: SomeNodeT, child_node_type: NodeType | None = None, recursive: bool = False
    ) -> tuple["SomeNodeT", ...] | list["SomeNodeT"]:
        """
        Collects all descendants as filtered in BFS order.
        If recursive, the child node type filter only applies to the first level.
        """
        raise NotImplementedError

    def iter_descendants(
        self, node: SomeNodeT, child_node_type: NodeType | None = None, recursive: bool = False
    ) -> Iterable[SomeNodeT]:
        """
        Iterate through filtered descendants in BFS order.
        If recursive, the child node type filter only applies to the first level.
        """
        return iter(self.collect_descendants(node, child_node_type, recursive))

    # utilities

    def __getitem__(self, item: IdT):
        return self.get(item)

    def __contains__(self, item: IdT):
        return self.get(item) is not None

    def set(self, nodes: Collection[SomeNodeT]):
        """Replaces all nodes in the graph"""
        self.clear()
        for node in nodes:
            self.add(node)

    def add_graph(self, graph: "NodeGraphBase[SomeNodeT, IdT]"):
        """Adds all nodes from another graph"""
        for node in graph.nodes:
            self.add(node)


class NodeGraph(NodeGraphBase[NodeT, UUID]):
    """A graph of Nodes with ids."""

    def __init__(self, nodes: Collection[NodeT] | "NodeGraph" = None):
        self.nodes_by_id: dict[UUID, NodeT] = {}
        self.nodes_by_ck: dict[UUID, NodeT] = {}  # most nodes have a ck as well
        self.nodes_by_parent_id_and_type: dict[tuple[UUID, NodeType], list[NodeT]] = defaultdict(
            list
        )

        if isinstance(nodes, list):
            for node in nodes or []:
                self.add(node)
        elif isinstance(nodes, NodeGraph):
            self.add_graph(nodes)

    @property
    def nodes(self) -> Collection[NodeT]:
        return self.nodes_by_ck.values()

    def copy(self):
        return NodeGraph(self)

    def get(self, node_id_or_ck: UUID) -> Optional[NodeT]:
        """Gets a node by id"""
        assert isinstance(node_id_or_ck, UUID), f"expected UUID, got {node_id_or_ck!r}"
        node = self.nodes_by_id.get(node_id_or_ck)
        if node is not None:
            return node
        return self.nodes_by_ck.get(node_id_or_ck)

    def clear(self):
        """Clear the graph"""
        self.nodes_by_id.clear()
        self.nodes_by_ck.clear()
        self.nodes_by_parent_id_and_type.clear()

    def add(self, node: NodeT):
        assert isinstance(node.id, UUID), f"cannot add {node!r} to {self!r} without id"
        if node.id in self.nodes_by_id:
            existing = self.nodes_by_id[node.id]
            raise ValueError(
                f"node {node!r} (id={node.id}) already exists in {self!r}: {existing!r} (id={existing.id})"
            )
        self.nodes_by_id[node.id] = node
        self.nodes_by_ck[node.ck] = node

        if node.parent_id is not None:
            self.nodes_by_parent_id_and_type[(node.parent_id, node.metatype)].append(node)

    def update(self, node: NodeT):
        assert isinstance(node.id, UUID), f"cannot add {node!r} to {self!r} without id"
        old = self.nodes_by_id.get(node.id)
        if old is None:
            raise ValueError(f"node {node!r} does not exist in {self!r}")
        self.nodes_by_id[node.id] = node
        self.nodes_by_ck[node.ck] = node

        # we can just update the parent->child reference and that will 'move' all descendants
        if old.parent_id != node.parent_id:
            if old.parent_id is not None:
                self.nodes_by_parent_id_and_type[(old.parent_id, old.metatype)].remove(old)
            if node.parent_id is not None:
                self.nodes_by_parent_id_and_type[(node.parent_id, node.metatype)].append(node)

    def remove(self, node: NodeT):
        from bench.language.node import CHILD_NODE_TYPES

        assert isinstance(node.id, UUID), f"cannot add {node!r} to {self!r} without id"

        queue = deque([node])
        if node.parent is not None:
            self.nodes_by_parent_id_and_type[(node.parent.id, node.metatype)].remove(node)
        while queue:
            node = queue.popleft()
            self.nodes_by_id.pop(node.id, None)
            self.nodes_by_ck.pop(node.ck, None)

            if node.__has_scope__:
                for child_type in CHILD_NODE_TYPES[node.metatype]:
                    children = self.nodes_by_parent_id_and_type.pop((node.id, child_type), ())
                    if len(children) > 0:
                        if CHILD_NODE_TYPES[child_type]:
                            queue.extend(children)
                        else:
                            for child in children:
                                self.nodes_by_id.pop(child.id, None)
                                self.nodes_by_ck.pop(child.ck, None)

    def find_roots(self) -> tuple[NodeDataT, ...]:
        return tuple(
            node
            for node in self.nodes_by_id.values()
            if node.parent_ptr is None or node.parent_ptr.id not in self.nodes_by_id
        )

    def collect_descendants(
        self,
        node: NodeT,
        child_node_type: NodeType | None = None,
        recursive: bool = False,
    ) -> tuple["NodeT", ...] | list["NodeT"]:
        from bench.language.node import CHILD_NODE_TYPES

        assert isinstance(node.id, UUID), f"expected Node, got {node!r}"
        if not node.__has_scope__:
            return ()
        if not recursive:
            if child_node_type is not None:  # best case
                return self.nodes_by_parent_id_and_type.get((node.id, child_node_type), ())
            else:
                descendants: list[NodeT] = []
                for child_type in CHILD_NODE_TYPES[node.metatype]:
                    children = self.nodes_by_parent_id_and_type.get((node.id, child_type), ())
                    if len(children) > 0:
                        descendants.extend(children)
                return descendants
        else:
            descendants: list[NodeT] = []
            if child_node_type:
                queue = deque()
                for child in self.nodes_by_parent_id_and_type.get((node.id, child_node_type), ()):
                    queue.append(child)
                    descendants.append(child)
            else:
                queue = deque((node,))
            while queue:
                current_nodes = queue.popleft()
                descendants.append(current_nodes)
                for child_type in CHILD_NODE_TYPES[current_nodes.metatype]:
                    children = self.nodes_by_parent_id_and_type.get(
                        (current_nodes.id, child_type), ()
                    )
                    queue.extend(children)
            return descendants


class NodeDataGraph(NodeGraphBase[NodeDataT, str]):
    """A NodeGraph for NodeData objects (strings for ids, parent_ptr)."""

    def __init__(self, nodes: Collection[NodeDataT] | "NodeDataGraph" = None):
        self.nodes_by_id: dict[str, NodeDataT] = {}
        self.nodes_by_ck: dict[str, NodeDataT] = {}  # *most* nodes have a 'ck'
        self.nodes_by_parent_id_and_type: dict[
            tuple[str, wire.NodeType], list[NodeDataT]
        ] = defaultdict(list)
        if isinstance(nodes, (list, tuple)):
            for node in nodes or []:
                self.add(node)
        elif isinstance(nodes, NodeDataGraph):
            self.add_graph(nodes)
        elif nodes is not None:
            raise ValueError(f"expected list or NodeDataGraph, got {nodes!r}")

    @property
    def nodes(self) -> Collection[NodeDataT]:
        return self.nodes_by_id.values()

    def copy(self):
        return NodeDataGraph(self)

    def get(self, node_id_or_ck: str) -> Optional[NodeDataT]:
        """Gets a node by id"""
        assert isinstance(node_id_or_ck, str), f"expected str, got {node_id_or_ck!r}"
        node = self.nodes_by_id.get(node_id_or_ck)
        if node is not None:
            return node
        return self.nodes_by_ck.get(node_id_or_ck)

    def clear(self):
        """Clear the graph"""
        self.nodes_by_id.clear()
        self.nodes_by_ck.clear()
        self.nodes_by_parent_id_and_type.clear()

    def add(self, node: NodeDataT):
        assert isinstance(node.id, str), f"cannot add {node!r} to {self!r} without id"
        if node.id in self.nodes_by_id:
            existing = self.nodes_by_id[node.id]
            raise ValueError(
                f"node {node!r} (id={node.id}) already exists in {self!r}: {existing!r} (id={existing.id})"
            )
        self.nodes_by_id[node.id] = node
        if hasattr(node, "ck"):
            self.nodes_by_ck[node.ck] = node
        if node.parent_ptr is not None:
            self.nodes_by_parent_id_and_type[(node.parent_ptr.id, node.metatype)].append(node)

    def update(self, node: NodeDataT):
        assert isinstance(node.id, str), f"cannot add {node!r} to {self!r} without id"
        old = self.nodes_by_id.get(node.id)
        if old is None:
            raise ValueError(f"node {node!r} does not exist in {self!r}")
        self.nodes_by_id[node.id] = node
        if hasattr(node, "ck"):
            self.nodes_by_ck[node.ck] = node
        if old.parent_ptr is not None:
            self.nodes_by_parent_id_and_type[(old.parent_ptr.id, old.metatype)].remove(old)
        if node.parent_ptr is not None:
            self.nodes_by_parent_id_and_type[(node.parent_ptr.id, node.metatype)].append(node)

    def remove(self, node: NodeDataT):
        assert isinstance(node.id, str), f"cannot add {node!r} to {self!r} without id"
        if node.parent_ptr is not None:
            self.nodes_by_parent_id_and_type[(node.parent_ptr.id, node.metatype)].remove(node)
        queue = deque([node])
        while queue:
            node = queue.popleft()
            self.nodes_by_id.pop(node.id, None)
            if hasattr(node, "ck"):
                self.nodes_by_ck.pop(node.ck, None)
            for child in self.nodes_by_parent_id_and_type.get((node.id, node.metatype), ()):
                self.nodes_by_id.pop(child.id, None)
                if child is not None:
                    queue.append(child)

    def find_roots(self) -> tuple[NodeDataT, ...]:
        return tuple(
            node
            for node in self.nodes_by_id.values()
            if node.parent_ptr is None or node.parent_ptr.id not in self.nodes_by_id
        )

    def collect_descendants(
        self, node: NodeDataT, child_node_type: NodeType | None = None, recursive: bool = False
    ) -> tuple["NodeDataT", ...] | list["NodeDataT"]:
        from bench.language.node import CHILD_NODE_TYPES

        assert isinstance(node.id, str), f"expected NodeData, got {Node!r}"
        if not recursive:
            if child_node_type is not None:
                return self.nodes_by_parent_id_and_type.get((node.id, child_node_type), ())
            else:
                descendants: list[NodeDataT] = []
                for child_type in CHILD_NODE_TYPES[node.metatype]:
                    descendants.extend(
                        self.nodes_by_parent_id_and_type.get((node.id, child_type), ())
                    )
                return descendants
        else:
            descendants: list[NodeDataT] = []
            if child_node_type:
                queue = deque(self.nodes_by_parent_id_and_type.get((node.id, child_node_type), ()))
            else:
                queue = deque((node,))
            while queue:
                current_nodes = queue.popleft()
                descendants.append(current_nodes)
                for child_type in CHILD_NODE_TYPES[current_nodes.metatype]:
                    queue.extend(
                        self.nodes_by_parent_id_and_type.get((current_nodes.id, child_type), ())
                    )
            return descendants

    def get_root(self, node: NodeDataT) -> NodeDataT:
        """Gets the root node for a given node"""
        root = node
        while root.parent_ptr is not None:
            root = self.nodes_by_id[root.parent_ptr.id]
        return root

    def walk_bfs(self, roots: list[NodeDataT] = None) -> Generator[NodeDataT, None, None]:
        """Walks the graph in breadth-first order"""
        from bench.language.node import CHILD_NODE_TYPES

        if roots is not None and len(roots) == 0:
            return
        queue = deque(roots or self.find_roots())
        while queue:
            current_nodes = queue.popleft()
            yield current_nodes
            for child_type in CHILD_NODE_TYPES[current_nodes.metatype]:
                queue.extend(
                    self.nodes_by_parent_id_and_type.get((current_nodes.id, child_type), ())
                )


class DetachedNodeGraph(NodeGraphBase[NodeT, UUID]):
    """
    A NodeGraph for nodes that may not have ids yet (are 'detached').
    We have a separate graph for this because wire nodes work with ids only (for parent),
     and we don't need to support all operations since it's only for detached nodes.
    """

    def __init__(self):
        self.nodes_by_ck: dict[UUID, "Node"] = {}
        self.nodes_by_parent_ck: dict[UUID, list["Node"]] = defaultdict(list)

    @property
    def nodes(self) -> Collection[NodeT]:
        return self.nodes_by_ck.values()

    def get(self, node_ck: UUID) -> Optional[NodeT]:
        """Gets a node by id"""
        assert isinstance(node_ck, UUID), f"expected UUID, got {node_ck!r}"
        return self.nodes_by_ck.get(node_ck)

    def clear(self):
        """Clear the graph"""
        self.nodes_by_ck.clear()
        self.nodes_by_parent_ck.clear()

    def add(self, node: "Node"):
        """Add a node to the graph (error if node already exists)"""
        if node.ck in self.nodes_by_ck and self.nodes_by_ck[node.ck] is not node:
            raise ValueError(f"node {node!r} (ck={node.ck}) already exists in {self!r}")
        self.nodes_by_ck[node.ck] = node

        if node.parent is not None:
            self.nodes_by_parent_ck[node.parent.ck].append(node)

    def update(self, node: "Node"):
        """Updates the node in this graph (must exist)"""
        existing = self.nodes_by_ck.get(node.ck)
        if existing is None:
            raise ValueError(f"node {node!r} (ck={node.ck}) does not exist in {self!r}")
        self.nodes_by_ck[node.ck] = node

        if existing.parent is not None and existing.parent in self.nodes_by_parent_ck:
            self.nodes_by_parent_ck[existing.parent_id].remove(existing)
        if node.parent is not None and node not in self.nodes_by_parent_ck[node.parent.ck]:
            self.nodes_by_parent_ck[node.parent.ck].append(node)

    def remove(self, node: "Node"):
        """Remove a node from the graph (incl. all descendants if recursive)"""
        descendants = self.collect_descendants(node, recursive=True, include_self=True)
        for descendant in descendants:
            descendant_ck = descendant.ck
            if descendant_ck in self.nodes_by_ck:
                self.nodes_by_ck.pop(descendant_ck)

            if descendant_ck in self.nodes_by_parent_ck:
                self.nodes_by_parent_ck.pop(descendant_ck)
            if descendant.parent and descendant.parent.ck in self.nodes_by_parent_ck:
                self.nodes_by_parent_ck[descendant.parent.ck].remove(descendant)

    def find_roots(self) -> tuple[NodeT, ...]:
        return tuple(
            node
            for node in self.nodes_by_ck.values()
            if node.parent is None or node.parent.ck not in self.nodes_by_ck
        )

    def collect_descendants(
        self,
        node: NodeT,
        child_node_type: NodeType | None = None,
        recursive: bool = False,
    ) -> tuple["NodeT", ...] | list["NodeT"]:
        assert isinstance(node.ck, UUID), f"expected UUID in node, got {node!r}"
        if not node.__has_scope__:
            return ()
        node_ck = node.ck
        if not recursive:
            if child_node_type is not None:
                return tuple(
                    child
                    for child in self.nodes_by_parent_ck.get(node_ck, ())
                    if child.metatype == child_node_type
                )
            else:
                return tuple(self.nodes_by_parent_ck.get(node_ck, ()))
        else:
            descendants: list[NodeT] = []
            if child_node_type:
                queue = deque(
                    child
                    for child in self.nodes_by_parent_ck.get(node_ck, ())
                    if child.metatype == child_node_type
                )
            else:
                queue = deque(self.nodes_by_parent_ck.get(node_ck, ()))
            while queue:
                current_nodes = queue.popleft()
                descendants.append(current_nodes)
                children = self.nodes_by_parent_ck.get(current_nodes.ck, ())
                if len(children) > 0:
                    queue.extend(children)
            return descendants


class NodeListBase(abc.ABC, Collection, Generic[NodeT]):
    """
    A list of node descendants for a parent's property.
    This is the primary way of adding, removing and accessing regular node relations.
    """

    __slots__ = ("_parent", "_property")

    def __init__(self, parent: "ScopeNode", property: "Property"):
        self._parent = parent
        self._property = property

    def __repr__(self):
        return f"<{self.__class__.__name__} {self._parent.absolute_path}.{self._property.name}: {self}>"

    def create(self, *args, **kwargs) -> NodeT:
        """Creates a new node in the list."""
        if len(args) == 1 and isinstance(args[0], Node):
            raise ValueError(f"cannot create {args[0]!r}, use append for existing nodes")
        from bench.language.node import NODE_CLASS_BY_TYPE

        node_cls = NODE_CLASS_BY_TYPE[self._property.reference_types[0]]
        # set new node status to source to prevent activation before it's appended
        if hasattr(node_cls, "new"):
            node = node_cls.new(*args, **kwargs, for_parent=self._parent, _status=NodeStatus.SOURCE)
        else:
            node = node_cls(*args, **kwargs, _status=NodeStatus.SOURCE)
        self.append(node)
        return node

    def append(self, node: NodeT) -> None:
        """
        Attaches a child node to a parent through a list. This is for users adding nodes.
        """
        raise NotImplementedError

    def extend(self, *nodes: Collection[NodeT]):
        """Attaches a list of child nodes to a parent. See append."""
        raise NotImplementedError

    def remove(self, node: NodeT):
        """Removes a child node from a parent. See append for reverse."""
        raise NotImplementedError

    def clear(self):
        """Removes all child nodes from a parent. See append for reverse."""
        raise NotImplementedError

    def set(self, nodes: Collection[NodeT]):
        """Replaces all child nodes of a parent."""
        self.clear()
        self.extend(*nodes)

    def get(self, some_id: str) -> Optional[NodeT]:
        """Gets a node by some id (as determined by the logic of the list)."""
        raise NotImplementedError


class NodeList(NodeListBase[NodeT]):
    # TODO @Cleanup @Architecture: use ReadQuery/WriteQuery in NodeList (with InMemoryGraphEngine to query)
    __slots__ = ("_child_node_type", "_flags")

    def __init__(self, parent: "ScopeNode", property: "Property"):
        super().__init__(parent, property)
        assert len(property.reference_types) == 1, f"cannot have many child types: {property!r}"
        self._child_node_type: NodeType = property.reference_types[0]
        self._flags = property.children_flags

    def __str__(self):
        return str(self.nodes)

    @property
    def nodes(self) -> tuple[NodeT, ...]:
        """Access the computed nodes"""
        return self._parent._root_graph.collect_descendants(
            self._parent, self._child_node_type, recursive=bool(self._flags & NRel.CUMULATIVE)
        )

    def _ok_bounds(
        self, after: NodeT = None, before: NodeT = None
    ) -> tuple[Optional[str], Optional[str]]:
        """Gets the order key bounds after the given (default to last)."""
        assert self._flags & NRel.ORDERED, f"cannot get order key for {self!r}"
        if after is not None:
            next_ok = nextn(
                n.order_key
                for n in self.nodes
                if n.order_key > after.order_key and n.parent == after.parent
            )
            return after.order_key, next_ok
        elif before is not None:
            last_ok = nextn(
                n.order_key
                for n in reversed(self.nodes)
                if n.order_key < before.order_key and n.parent == before.parent
            )
            return last_ok, before.order_key
        else:
            last_ok = nextn((n.order_key for n in reversed(self.nodes) if n.parent == self._parent))
            return last_ok, None

    def append(self, n: NodeT, after: NodeT = None, before: NodeT = None) -> tuple[NodeT, ...]:
        assert isinstance(n, Node), f"cannot append {n!r} to {self!r}"
        if n.parent is not None:
            raise ValueError(f"cannot attach {n!r} to {self!r}: attached to {n.parent!r}")

        # assign ids if newly attached to the package (ids are derived from ck + package)
        if not n.is_attached and self._parent.package:
            package_id = self._parent.package.id
            for n in n._walk_rec():
                if n.id is None:
                    n._assign_id(package_id)
        # update parent after updating ids (the above walks graph, which is effectd here)
        n.parent = self._parent
        # validate node now that it has a parent (while in session)
        if self._parent._session is not None:
            n._validate_self(n.__tracked_properties__.keys(), on_invalid=on_invalid_raise)

        # add node to parent graph
        if n.__has_scope__ and n._local_graph is not None:
            # subsume if previously detached (ignores out of line nodes)
            added = n._local_graph.collect_descendants(n, recursive=True)
            n._local_graph.update(n)  # parent updated
            self._parent._root_graph.add_graph(n._local_graph)
            n._local_graph = None
        else:
            added = (n,)
            self._parent._root_graph.add(n)

        # assign order key to ordered nodes
        if self._flags & NRel.ORDERED and n.order_key is None:
            n.order_key = generate_key_between(*self._ok_bounds(after, before))
        # 'create' node in session if it's attached
        if self._parent._session and self._parent.is_attached:
            self._parent._session.create(*added)

        return added

    def extend(self, *nodes: NodeT, after: NodeT = None, before: NodeT = None) -> None:
        if not nodes:
            return

        # pre-assign order keys since we don't trigger between appends (meaning last_ok is wrong)
        if self._flags & NRel.ORDERED:
            oks = generate_n_keys_between(*self._ok_bounds(after, before), n=len(nodes))
            for node, ok in zip(nodes, oks):
                node.order_key = ok

        for node in nodes:
            self.append(node)

    def remove(self, n: NodeT):
        if self._parent._session:
            self._parent.session.delete(n)
        self._parent._root_graph.remove(n)
        n.parent = None

    def clear(self):
        if not self.nodes:
            return
        removed = tuple(self.nodes)
        for n in removed:
            self.remove(n)

    def get(self, some_id: str) -> Optional[NodeT]:
        if not (self._flags & NRel.KEYED) and not (self._flags & NRel.NAMED):
            raise ValueError(f"cannot get {some_id!r} from {self!r}")
        for child in self.nodes:
            if (self._flags & NRel.KEYED and child.dynamic_key == some_id) or (
                self._flags & NRel.NAMED and (child.name == some_id or child.py_ident == some_id)
            ):
                return child
        return None

    def __bool__(self):
        return len(self.nodes) > 0

    def __contains__(self, obj: object) -> bool:
        # special case to unwrap key (e.g. for tagging/tag objects)
        if self._flags & NRel.KEYED and hasattr(obj, "dynamic_key"):
            obj = obj.dynamic_key
        if isinstance(obj, str) and (self._flags & NRel.KEYED or self._flags & NRel.NAMED):
            return self.get(obj) is not None
        elif isinstance(obj, Node):
            if obj.metatype != self._property.reference_types[0]:
                raise TypeError(f"{self!r} cannot contain {obj!r}")
            return obj in self.nodes
        else:
            return False

    def __getitem__(self, item: int | slice | str) -> NodeT | list[NodeT]:
        if isinstance(item, int):
            return self.nodes[item]
        elif isinstance(item, slice):
            return self.nodes[item]
        elif isinstance(item, str):
            return self.get(item)
        else:
            raise TypeError(f"invalid index for {self!r}: {item} ({type(item)})")

    def __getattr__(self, item):
        if item.startswith("_"):
            return super().__getattribute__(item)
        node = self.get(item)
        if node is None:
            raise AttributeError(f"no node '{item}' in {self!r}")
        return node

    def __iter__(self) -> Iterator[NodeT]:
        yield from self.nodes

    def __len__(self) -> int:
        return len(self.nodes)

    def __eq__(self, other: object) -> bool:
        if isinstance(other, NodeList):
            return self.nodes == other.nodes
        elif isinstance(other, list):
            return self.nodes == other
        else:
            return False
