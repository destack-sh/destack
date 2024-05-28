import abc
from collections import defaultdict, deque
from itertools import chain
from typing import (
    TYPE_CHECKING,
    Any,
    Collection,
    Generic,
    Iterable,
    Iterator,
    Optional,
    TypeVar,
    Union,
    cast,
    overload,
)
from uuid import UUID

from more_itertools import first

from bench.language.const import EMPTY_LIST, EditType, NodeType, ReferenceKind
from bench.language.setup import CHILD_NODE_TYPES, NODE_CLASS_BY_TYPE, STRUCT_CLASS_BY_TYPE
from bench.language.validation import on_invalid_raise
from bench.proto import wire
from bench.proto.wire import AnyNodeData, EditData
from bench.utils.casing import Casing, to_casing
from bench.utils.fractional import get_key_bounds, get_order_key, get_order_keys
from bench.utils.func import IdEnum, to_uuid

if TYPE_CHECKING:
    # noinspection PyUnresolvedReferences
    from bench.language import Field, Node, Object, Property, ReadOptions, Struct

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

    def extend(self, nodes: Collection[SomeNodeT]):
        """Adds all nodes to the graph"""
        for node in nodes:
            self.add(node)

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

    def __init__(self, nodes: Collection[NodeT] | "NodeGraph" | None = None):
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
        assert isinstance(node.id, UUID), f"cannot add {node!r} to {self!r} with id {node.id!r}"
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
        assert isinstance(node.id, UUID), f"cannot add {node!r} to {self!r} with id {node.id!r}"
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
        assert isinstance(node.id, UUID), f"cannot add {node!r} to {self!r} with id {node.id!r}"

        queue = deque([node])
        if node.parent is not None:
            children = self.nodes_by_parent_id_and_type[(node.parent.id, node.metatype)]
            assert node in children, f"{node!r} not in {children!r} of {node.parent!r}"
            children.remove(node)
        while queue:
            node = queue.popleft()
            self.nodes_by_id.pop(node.id, None)
            self.nodes_by_ck.pop(node.ck, None)

            for child_type in CHILD_NODE_TYPES[node.metatype]:
                children = self.nodes_by_parent_id_and_type.pop((node.id, child_type), ())
                if len(children) > 0:
                    if CHILD_NODE_TYPES[child_type]:
                        queue.extend(children)
                    else:
                        for child in children:
                            self.nodes_by_id.pop(child.id, None)
                            self.nodes_by_ck.pop(child.ck, None)

    def find_roots(self) -> tuple[NodeT, ...]:
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
    ) -> list["NodeT"]:
        assert isinstance(node.id, UUID), f"expected Node, got {node!r}"
        if not CHILD_NODE_TYPES[node.metatype]:
            return EMPTY_LIST
        if not recursive:
            if child_node_type is not None:  # best case
                return self.nodes_by_parent_id_and_type.get((node.id, child_node_type), [])
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
            else:
                queue = deque()
                for child_type in CHILD_NODE_TYPES[node.metatype]:
                    children = self.nodes_by_parent_id_and_type.get((node.id, child_type), ())
                    queue.extend(children)
            while queue:
                cur = queue.popleft()
                descendants.append(cur)
                for child_type in CHILD_NODE_TYPES[cur.metatype]:
                    children = self.nodes_by_parent_id_and_type.get((cur.id, child_type), ())
                    queue.extend(children)
            return descendants


class NodeDataGraph(NodeGraphBase[NodeDataT, str]):
    """A NodeGraph for NodeData objects (strings for ids, parent_ptr)."""

    def __init__(self, nodes: Collection[NodeDataT] | None = None):
        self.nodes_by_id: dict[str, NodeDataT] = {}
        self.nodes_by_ck: dict[str, NodeDataT] = {}  # *most* nodes have a 'ck'
        self.nodes_by_parent_id_and_type: dict[
            # NOTE: typing the key broad like this is to avoid casting all the time
            tuple[Any, wire.NodeType | wire.ObjectType | NodeType],
            list[NodeDataT],
        ] = defaultdict(list)
        if isinstance(nodes, Collection):
            for node in nodes:
                self.add(node)
        elif nodes is not None:
            raise ValueError(f"expected nodes, got {nodes!r}")

    @property
    def nodes(self) -> Collection[NodeDataT]:
        return self.nodes_by_id.values()

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
        assert isinstance(node.id, str), f"cannot add {node!r} to {self!r} with id {node.id!r}"
        if node.id in self.nodes_by_id:
            existing = self.nodes_by_id[node.id]
            raise ValueError(
                f"node {node!r} (id={node.id}) already exists in {self!r}: {existing!r} (id={existing.id})"
            )
        self.nodes_by_id[node.id] = node
        if hasattr(node, "ck"):
            self.nodes_by_ck[getattr(node, "ck")] = node
        if node.parent_ptr is not None:
            parent_id = cast(str, node.parent_ptr.id)
            self.nodes_by_parent_id_and_type[(parent_id, node.metatype)].append(node)

    def update(self, node: NodeDataT):
        assert isinstance(node.id, str), f"cannot add {node!r} to {self!r} with id {node.id!r}"
        old = self.nodes_by_id.get(node.id)
        if old is None:
            raise ValueError(f"node {node!r} does not exist in {self!r}")
        self.nodes_by_id[node.id] = node
        if hasattr(node, "ck"):
            self.nodes_by_ck[getattr(node, "ck")] = node
        if old.parent_ptr is not None:
            self.nodes_by_parent_id_and_type[(old.parent_ptr.id, old.metatype)].remove(old)
        if node.parent_ptr is not None:
            self.nodes_by_parent_id_and_type[(node.parent_ptr.id, node.metatype)].append(node)

    def remove(self, node: NodeDataT):
        assert isinstance(node.id, str), f"cannot add {node!r} to {self!r} with id {node.id!r}"
        if node.parent_ptr is not None:
            self.nodes_by_parent_id_and_type[(node.parent_ptr.id, node.metatype)].remove(node)
        queue = deque([node])
        while queue:
            node = queue.popleft()
            self.nodes_by_id.pop(node.id, None)
            if hasattr(node, "ck"):
                self.nodes_by_ck.pop(getattr(node, "ck"), None)
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
    ) -> list["NodeDataT"]:
        assert isinstance(node.id, str), f"expected NodeData, got {Node!r}"
        if not recursive:
            if child_node_type is not None:
                return self.nodes_by_parent_id_and_type.get((node.id, child_node_type), [])
            else:
                descendants: list[NodeDataT] = []
                for child_type in CHILD_NODE_TYPES[cast(NodeType, node.metatype)]:
                    descendants.extend(
                        self.nodes_by_parent_id_and_type.get((node.id, child_type), ())
                    )
                return descendants
        else:
            descendants: list[NodeDataT] = []
            if child_node_type:
                queue = deque(self.nodes_by_parent_id_and_type.get((node.id, child_node_type), ()))
            else:
                queue = deque()
                for child_type in CHILD_NODE_TYPES[cast(NodeType, node.metatype)]:
                    queue.extend(self.nodes_by_parent_id_and_type.get((node.id, child_type), ()))
            while queue:
                cur = queue.popleft()
                descendants.append(cur)
                for child_type in CHILD_NODE_TYPES[cast(NodeType, cur.metatype)]:
                    queue.extend(self.nodes_by_parent_id_and_type.get((cur.id, child_type), ()))
            return descendants

    def get_root(self, node: NodeDataT) -> NodeDataT:
        """Gets the root node for a given node"""
        root = node
        while root.parent_ptr is not None:
            root = self.nodes_by_id[cast(str, root.parent_ptr.id)]
        return root


class DetachedNodeGraph(NodeGraphBase[NodeT, UUID]):
    """
    A NodeGraph for nodes that may not have ids yet (are 'detached').
    We have a separate graph for this because wire nodes work with ids only (for parent),
     and we don't need to support all operations since it's only for detached nodes.
    """

    def __init__(self):
        self.nodes_by_ck: dict[UUID, NodeT] = {}
        self.nodes_by_parent_ck: dict[UUID, list[NodeT]] = defaultdict(list)

    @property
    def nodes(self) -> Collection[NodeT]:
        return self.nodes_by_ck.values()

    def get(self, node_id_or_ck: UUID) -> Optional[NodeT]:
        """Gets a node by id"""
        assert isinstance(node_id_or_ck, UUID), f"expected UUID, got {node_id_or_ck!r}"
        return self.nodes_by_ck.get(node_id_or_ck)

    def clear(self):
        """Clear the graph"""
        self.nodes_by_ck.clear()
        self.nodes_by_parent_ck.clear()

    def add(self, node: NodeT):
        """Add a node to the graph (error if node already exists)"""
        if node.ck in self.nodes_by_ck and self.nodes_by_ck[node.ck] is not node:
            raise ValueError(f"node {node!r} (ck={node.ck}) already exists in {self!r}")
        self.nodes_by_ck[node.ck] = node

        if node.parent is not None:
            self.nodes_by_parent_ck[node.parent.ck].append(node)

    def update(self, node: NodeT):
        """Updates the node in this graph (must exist)"""
        existing = self.nodes_by_ck.get(node.ck)
        if existing is None:
            raise ValueError(f"node {node!r} (ck={node.ck}) does not exist in {self!r}")
        self.nodes_by_ck[node.ck] = node

        if existing.parent is not None and existing.parent in self.nodes_by_parent_ck:
            self.nodes_by_parent_ck[existing.parent.ck].remove(existing)
        if node.parent is not None and node not in self.nodes_by_parent_ck[node.parent.ck]:
            self.nodes_by_parent_ck[node.parent.ck].append(node)

    def remove(self, node: NodeT):
        """Remove a node from the graph (incl. all descendants if recursive)"""
        descendants = self.collect_descendants(node, recursive=True)
        for descendant in chain((node,), descendants):
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
    ) -> list["NodeT"]:
        assert isinstance(node.ck, UUID), f"expected UUID in node, got {node!r}"
        if not CHILD_NODE_TYPES[node.metatype]:
            return []
        node_ck = node.ck
        if not recursive:
            if child_node_type is not None:
                return [
                    child
                    for child in self.nodes_by_parent_ck.get(node_ck, EMPTY_LIST)
                    if child.metatype == child_node_type
                ]
            else:
                return self.nodes_by_parent_ck.get(node_ck, EMPTY_LIST)
        else:
            descendants: list[NodeT] = []
            if child_node_type:
                queue = deque(
                    child
                    for child in self.nodes_by_parent_ck.get(node_ck, EMPTY_LIST)
                    if child.metatype == child_node_type
                )
            else:
                queue = deque(self.nodes_by_parent_ck.get(node_ck, EMPTY_LIST))
            while queue:
                current_node = queue.popleft()
                descendants.append(current_node)
                children = self.nodes_by_parent_ck.get(current_node.ck, ())
                if len(children) > 0:
                    queue.extend(children)
            return descendants


class NodeDict:
    """A simple graph-like wrapper for a dict of nodes that has some of the same methods."""

    def __init__(self, nodes_by_id: dict[UUID, "Node"]):
        self._nodes_by_id = nodes_by_id

    def __str__(self):
        return f"{len(self._nodes_by_id)} nodes"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}>"

    @property
    def nodes(self):
        return self._nodes_by_id.values()

    def __getitem__(self, item: UUID) -> "Node":
        return self._nodes_by_id[item]

    def __contains__(self, item: UUID) -> bool:
        return item in self._nodes_by_id

    def get(self, item: UUID) -> "Node":
        return self._nodes_by_id[item]


NodeGraphLike = Union[NodeGraph["Node"], DetachedNodeGraph["Node"], NodeDict]


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


class NodeList(abc.ABC, Collection[NodeT], Generic[NodeT]):
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
    def nodes(self) -> tuple[NodeT, ...] | list[NodeT]:
        raise NotImplementedError

    def create(self, **kwargs) -> NodeT:
        """Creates a new node in the list."""
        from bench.language.node import NODE_CLASS_BY_TYPE

        node_metatype = cast(list[NodeType], self._property.reference_nodes)[0]
        node_cls = NODE_CLASS_BY_TYPE[node_metatype]
        # auto-generate name if required and not given :AutoNaming
        if "name" in node_cls.__properties__ and "name" not in kwargs:
            kwargs["name"] = generate_node_name(node_metatype, kwargs.get("type"), self)
        # set new node status to source to prevent activation before it's appended
        node = node_cls(**kwargs)
        node = cast(NodeT, node)
        self.append(node)
        return node

    def append(self, node: NodeT) -> None:
        """Attaches a child node to a parent through a list."""
        raise NotImplementedError

    def extend(self, *nodes: NodeT):
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

    def get(self, key: UUID | str | int) -> NodeT | None:
        """Gets a node by some key (id/ck, actual name or identifier)."""
        raise NotImplementedError

    def __contains__(self, obj: object | NodeT | str | UUID) -> bool:
        """Checks if a node is in the list."""
        if type(obj) is str or type(obj) is UUID:
            return self.get(obj) is not None
        else:
            return obj in self

    @overload
    def __getitem__(self, item: str) -> Optional[NodeT]: ...
    @overload
    def __getitem__(self, item: UUID) -> Optional[NodeT]: ...
    @overload
    def __getitem__(self, item: int) -> NodeT: ...
    @overload
    def __getitem__(self, item: slice) -> list[NodeT]: ...
    def __getitem__(self, item: Union[str, UUID, int, slice]):
        """Gets a node by index or name."""
        if isinstance(item, (str, UUID)):
            return self.get(item)
        else:
            return self.nodes[item]

    def __getattr__(self, item: str) -> NodeT:
        """Gets a node by name."""
        node = self.get(item)
        if node is None:
            raise AttributeError(f"{self!r} has no node {item!r}")
        return node

    def tolist(self) -> list[NodeT]:
        return list(self)


class GraphNodeList(NodeList[NodeT]):
    # TODO :Cleanup :Architecture: use ReadQuery/WriteQuery in NodeList?
    #  (with InMemoryGraphEngine to query)
    __slots__ = ("_child_node_type", "_flags")

    def __init__(self, parent: "Node", property: "Property"):
        super().__init__(parent, property)
        assert (
            property.reference_nodes and len(property.reference_nodes) == 1
        ), f"cannot have many child types: {property!r}"
        self._child_node_type: NodeType = property.reference_nodes[0]
        self._flags = property.reference_flags

    def __str__(self):
        return str(self.nodes)

    def get(self, key: UUID | str | int) -> NodeT | None:
        if isinstance(key, UUID):
            return cast(NodeT, self._parent._graph.get(key))
        elif isinstance(key, str):
            return first(
                (n for n in self.nodes if n.py_ident == key or getattr(n, "name", None) == key),
                None,
            )
        else:
            return self.nodes[key]

    @property
    def nodes(self) -> tuple[NodeT, ...] | list[NodeT]:
        """Access the computed nodes"""
        descendants = self._parent._graph.collect_descendants(
            self._parent, self._child_node_type, recursive=False
        )
        if (
            len(descendants) > 1
            and "order_key" in NODE_CLASS_BY_TYPE[self._child_node_type].__properties__
        ):
            descendants.sort(key=lambda n: n.order_key)  # type: ignore
        return cast(list[NodeT], descendants)

    def append(  # type: ignore
        self, node: NodeT, after: NodeT | None = None, before: NodeT | None = None
    ) -> tuple[NodeT, ...]:
        from bench.language.node import Node

        assert isinstance(node, Node), f"cannot append {node!r} to {self!r}"
        if node.parent is not None:
            raise ValueError(f"cannot attach {node!r} to {self!r}: attached to {node.parent!r}")

        # assign ids if newly attached to the package (ids are derived from ck + package)
        if "ck" in node.__properties__ and not node.is_attached and self._parent.is_attached:
            package_id = self._parent.package.id
            for n in node._walk_descendants():
                if n.id is None:
                    n._assign_id(package_id)

        node.parent = self._parent
        if self._parent._session is not None:
            node._validate_self((), invalid=on_invalid_raise)

        # add node (and descendants) to this parent's graph
        if node._graph is not self._parent._graph:
            added = node._graph.collect_descendants(node, recursive=True)
            added = (*added, node)
            node._graph.update(node)  # parent updated
            self._parent._graph.add_graph(node._graph)
            node._graph = self._parent._graph
        else:
            added = (node,)
            self._parent._graph.add(node)

        # assign order key to ordered nodes
        if hasattr(node, "order_key") and getattr(node, "order_key") is None:
            node.order_key = get_order_key(*get_key_bounds(self.nodes, after, before))

        # 'create' node in session if it's attached
        if self._parent._session and self._parent.is_attached:
            self._parent._session.create(*added)
            self._parent._session.track_many(*added)

        return cast(tuple[NodeT, ...], added)

    def extend(
        self, *nodes: NodeT, after: NodeT | None = None, before: NodeT | None = None
    ) -> None:  # type: ignore
        if not nodes:
            return

        # pre-assign order keys since we don't trigger between appends (meaning last_ok is wrong)
        if hasattr(nodes[0], "order_key") and getattr(nodes[0], "order_key") is None:
            oks = get_order_keys(*get_key_bounds(self.nodes, after, before), n=len(nodes))
            for node, ok in zip(nodes, oks):
                node.order_key = ok

        for node in nodes:
            self.append(node)

    def remove(self, n: NodeT):  # type: ignore
        if self._parent._session is not None:
            self._parent._session.soft_delete(n)
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

    def __iter__(self) -> Iterator[NodeT]:
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


ValueParentT = TypeVar("ValueParentT", bound=Union["Object", "Struct", "Node"])
ValueT = TypeVar("ValueT", bound=Union["Object", "Struct", "Property"])
ValueProperty = Union["Property", "Field"]


class ValueList(list, Generic[ValueParentT]):
    """
    A list of Values or Value-like Structs (with local identity, so can't be inlined).
    Unlike a NodeList, value lists are actual lists and not computed on access.
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

    def append(self, item: ValueT, after: ValueT | None = None, before: ValueT | None = None):
        if not self.is_property_reference:
            item = item._move_to(self.parent, self.parent_prop)  # type: ignore
        super().append(item)
        if self.is_ordered:
            cast(Union["Object", "Struct"], item).order_key = get_order_key(
                *get_key_bounds(self, after, before)
            )
        self.parent._updated_self((self.ancestor_prop,))

    def extend(self, items: Collection[ValueT]):  # type: ignore
        super().extend(items)
        if not self.is_property_reference:
            values = cast(list[Union["Object", "Struct"]], items)
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
            for v in cast(list[Union["Object", "Struct"]], values)
        ):
            values = [v._copy_to(parent, parent_prop) for v in values]  # type: ignore
        return ValueList(parent, parent_prop, ancestor_prop, values)


# poor mans filters, see FilterNodeGraph in bench-web
_INCLUDE_ALL_EDIT_TYPE_REMAP: dict[EditType, EditType] = {
    EditType.ARCHIVE: EditType.UPDATE,
    EditType.UNARCHIVE: EditType.UPDATE,
    EditType.SOFT_DELETE: EditType.UPDATE,
    EditType.RESTORE: EditType.UPDATE,
    EditType.DELETE: EditType.UPDATE,
}
_INCLUDE_HIDDEN_EDIT_TYPE_REMAP: dict[EditType, EditType] = {
    EditType.ARCHIVE: EditType.UPDATE,
    EditType.UNARCHIVE: EditType.UPDATE,
    EditType.SOFT_DELETE: EditType.UPDATE,
    EditType.RESTORE: EditType.UPDATE,
}
_EXCLUDE_HIDDEN_EDIT_TYPE_REMAP: dict[EditType, EditType] = {
    EditType.ARCHIVE: EditType.DELETE,
    EditType.UNARCHIVE: EditType.CREATE,
    EditType.SOFT_DELETE: EditType.DELETE,
    EditType.RESTORE: EditType.CREATE,
}


def edit_graph(
    graph: NodeGraph["Node"] | DetachedNodeGraph["Node"],
    edits: Collection[EditData],
    options: "ReadOptions | None",
) -> None:
    """Applies the edits to the graph (in place!)."""

    from bench.proto import wiring

    if options is None:
        options = ReadOptions()

    for edit in edits:
        node_data = wiring.unwrap_some_node(edit.node)
        node_id = to_uuid(node_data.id)
        if node_id is None:
            raise ValueError(f"invalid node id in edit {edit!r}: {node_data!r}")

        edit_type = cast(EditType, edit.type)  # remap edit according to read options
        if options.include_hidden:
            edit_type = _INCLUDE_HIDDEN_EDIT_TYPE_REMAP.get(edit_type, edit_type)
        else:
            edit_type = _EXCLUDE_HIDDEN_EDIT_TYPE_REMAP.get(edit_type, edit_type)

        if edit_type == EditType.CREATE or (edit_type == EditType.UPSERT and node_id not in graph):
            if node_data.parent_ptr is not None:
                parent = graph.get(UUID(node_data.parent_ptr.id))
            else:
                parent = None
            node = wiring.unpack_node(node_data, parent)
            graph.add(node)
        elif edit_type == EditType.DELETE:
            node = graph.get(node_id)
            assert node is not None, f"missing node {node_id!r} for delete: {edit!r}"
            graph.remove(node)
        elif edit_type == EditType.MOVE:
            if node_data.parent_ptr is not None:
                new_parent = graph.get(UUID(node_data.parent_ptr.id))
            else:
                new_parent = None
            node = graph.get(node_id)
            assert node is not None, f"missing node {node_id!r} for move: {edit!r}"
            node.parent = new_parent
        elif edit_type == EditType.UPDATE:
            node = graph.get(node_id)
            assert node is not None, f"missing node {node_id!r} for update: {edit!r}"
            for prop_id in edit.properties:
                prop = node.__properties_by_id__[prop_id]
                prop = prop.reference_wired_ptr or prop
                updated_value_data = getattr(node_data, prop.name)
                updated_value = wiring.unpack_struct_prop(prop, updated_value_data)
                setattr(node, prop.name, updated_value)


def edit_data_graph(
    graph: NodeDataGraph[AnyNodeData],
    options: "ReadOptions",
    edits: Collection[EditData],
    *,
    keep_all: bool = False,
    update_nodes_in_place: bool = False,
) -> None:
    """Applies the edits to the data graph."""

    from bench.proto import wiring

    for edit in edits:
        node_data = wiring.unwrap_some_node(edit.node)

        edit_type = cast(EditType, edit.type)  # remap edit according to read options
        if keep_all:
            edit_type = _INCLUDE_ALL_EDIT_TYPE_REMAP.get(edit_type, edit_type)
        elif options.include_hidden:
            edit_type = _INCLUDE_HIDDEN_EDIT_TYPE_REMAP.get(edit_type, edit_type)
        else:
            edit_type = _EXCLUDE_HIDDEN_EDIT_TYPE_REMAP.get(edit_type, edit_type)

        if edit_type == EditType.CREATE or (
            edit_type == EditType.UPSERT and node_data.id not in graph
        ):
            graph.add(node_data)
        elif edit_type == EditType.DELETE:
            graph.remove(node_data)
        else:  # some update
            node_cls = NODE_CLASS_BY_TYPE[cast(NodeType, edit.node_type)]
            if edit_type == EditType.UPDATE:
                properties = edit.properties
            elif edit_type == EditType.MOVE:
                properties = (cast("Property", node_cls.parent).id,)
            elif edit_type in (EditType.ARCHIVE, EditType.UNARCHIVE):
                properties = (cast("Property", node_cls.archived_at).id,)
            elif edit_type in (EditType.SOFT_DELETE, EditType.RESTORE):
                properties = (cast("Property", node_cls.deleted_at).id,)
            else:
                raise ValueError(f"unexpected edit type: {edit_type.name}")
            existing_node = graph.get(node_data.id)
            assert existing_node is not None, f"missing node for update: {edit}"
            if not update_nodes_in_place:
                existing_node = wiring.copy_data(existing_node)
            for prop_id in properties:
                prop = node_cls.__properties_by_id__[prop_id]
                prop = prop.reference_wired_ptr or prop
                updated_value = getattr(node_data, prop.name)
                setattr(existing_node, prop.name, updated_value)
            graph.update(existing_node)
