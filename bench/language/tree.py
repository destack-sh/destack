import abc
from collections import defaultdict, deque
from typing import (
    TYPE_CHECKING,
    Collection,
    Generator,
    Generic,
    Iterable,
    Optional,
    TypeVar,
    Union,
)
from uuid import UUID

from bench.language.const import NodeType
from bench.proto.wire import AnyNodeData

if TYPE_CHECKING:
    from bench.language import Node

NodeT = TypeVar("NodeT", bound="Node")
NodeDataT = TypeVar("NodeDataT", bound=AnyNodeData)
SomeNodeT = TypeVar("SomeNodeT")
IdT = TypeVar("IdT", bound=Union[UUID, str])


class NodeTreeBase(abc.ABC, Generic[SomeNodeT, IdT]):
    def __str__(self):
        return f"{len(self.nodes)} nodes"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}>"

    @property
    def nodes(self) -> Collection[SomeNodeT]:
        raise NotImplementedError

    def copy(self) -> "NodeTreeBase[SomeNodeT, IdT]":
        raise NotImplementedError

    def get(self, node_id_or_ck: IdT) -> Optional[SomeNodeT]:
        """Gets a node by id or ck"""
        raise NotImplementedError

    def clear(self):
        """Clear the tree"""
        raise NotImplementedError

    def add(self, node: "SomeNodeT"):
        """Add a node to the tree (error if node already exists, *no* descendants)"""
        raise NotImplementedError

    def update(self, node: "SomeNodeT"):
        """Updates an existing node in this tree (must exist)"""
        raise NotImplementedError

    def remove(self, node: "SomeNodeT"):
        """Remove a node from the tree (incl. all descendants)"""
        raise NotImplementedError

    def find_roots(self) -> tuple[SomeNodeT, ...]:
        raise NotImplementedError

    def find_root(self) -> Optional[SomeNodeT]:
        roots = self.find_roots()
        if len(roots) > 1:
            raise ValueError(f"expected 0 or 1 root nodes, got {roots}")
        return roots[0] if roots else None

    def collect_descendants(
        self, node: SomeNodeT, node_type: NodeType | None = None, recursive: bool = False
    ) -> tuple["NodeT", ...] | list["NodeT"]:
        """Collects all descendants as filtered in BFS order"""
        raise NotImplementedError

    def iter_descendants(
        self, node: SomeNodeT, node_type: NodeType | None = None, recursive: bool = False
    ) -> Iterable[NodeT]:
        """Iterate through filtered descendants in BFS order"""
        return iter(self.collect_descendants(node, node_type, recursive))

    # utilities

    def __getitem__(self, item: IdT):
        return self.get(item)

    def __contains__(self, item: IdT):
        return self.get(item) is not None

    def set(self, nodes: Collection[NodeT]):
        """Replaces all nodes in the tree"""
        self.clear()
        for node in nodes:
            self.add(node)

    def add_tree(self, tree: "NodeTreeBase[NodeT, IdT]"):
        """Adds all nodes from another tree"""
        for node in tree.nodes:
            self.add(node)


class NodeTree(NodeTreeBase[NodeT, UUID]):
    """
    An indexed tree of package nodes (UUIDs for ids, parent_ids).
    This is the backing tree to most live nodes, so we optimize access a bit.
    """

    def __init__(self, nodes: Collection[NodeT] | "NodeTree" = None):
        self.nodes_by_id: dict[UUID, NodeT] = {}
        self.nodes_by_ck: dict[UUID, NodeT] = {}  # most nodes have a ck as well
        self.nodes_by_parent_id_and_type: dict[tuple[UUID, NodeType], list[NodeT]] = defaultdict(
            list
        )

        if isinstance(nodes, list):
            for node in nodes or []:
                self.add(node)
        elif isinstance(nodes, NodeTree):
            self.add_tree(nodes)

    @property
    def nodes(self) -> Collection[NodeT]:
        return self.nodes_by_ck.values()

    def copy(self):
        return NodeTree(self)

    def get(self, node_id_or_ck: UUID) -> Optional[NodeT]:
        """Gets a node by id"""
        assert isinstance(node_id_or_ck, UUID), f"expected UUID, got {node_id_or_ck!r}"
        node = self.nodes_by_id.get(node_id_or_ck)
        if node is not None:
            return node
        return self.nodes_by_ck.get(node_id_or_ck)

    def clear(self):
        """Clear the tree"""
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
        node_type: NodeType | None = None,
        recursive: bool = False,
    ) -> tuple["NodeT", ...] | list["NodeT"]:
        from bench.language.node import CHILD_NODE_TYPES

        assert isinstance(node.id, UUID), f"expected Node, got {node!r}"
        if not node.__has_scope__:
            return ()
        if not recursive:
            if node_type is not None:  # best case
                return self.nodes_by_parent_id_and_type.get((node.id, node_type), ())
            else:
                descendants: list[NodeT] = []
                for child_type in CHILD_NODE_TYPES[node.metatype]:
                    children = self.nodes_by_parent_id_and_type.get((node.id, child_type), ())
                    if len(children) > 0:
                        descendants.extend(children)
                return descendants
        else:
            descendants: list[NodeT] = []
            queue = deque([node])
            while queue:
                current_node = queue.popleft()
                for child_type in CHILD_NODE_TYPES[current_node.metatype]:
                    children = self.nodes_by_parent_id_and_type.get(
                        (current_node.id, child_type), ()
                    )
                    if len(children) > 0:
                        if node_type is None or child_type == node_type:
                            descendants.extend(children)
                        queue.extend(children)
            return descendants


class NodeDataTree(NodeTreeBase[NodeDataT, str]):
    """A NodeTree for NodeData objects (strings for ids, parent_ptr)."""

    def __init__(self, nodes: Collection[NodeDataT] | "NodeDataTree" = None):
        self.nodes_by_id: dict[str, NodeDataT] = {}
        self.nodes_by_ck: dict[str, NodeDataT] = {}  # *most* nodes have a ck as well
        self.node_ids_by_parent_id: dict[str, list[str]] = {}
        if isinstance(nodes, list):
            for node in nodes or []:
                self.add(node)
        elif isinstance(nodes, NodeDataTree):
            self.add_tree(nodes)

    @property
    def nodes(self) -> Collection[NodeDataT]:
        return self.nodes_by_id.values()

    def copy(self):
        return NodeDataTree(self)

    def get(self, node_id_or_ck: str) -> Optional[NodeDataT]:
        """Gets a node by id"""
        assert isinstance(node_id_or_ck, str), f"expected str, got {node_id_or_ck!r}"
        node = self.nodes_by_id.get(node_id_or_ck)
        if node is not None:
            return node
        return self.nodes_by_ck.get(node_id_or_ck)

    def clear(self):
        """Clear the tree"""
        self.nodes_by_id.clear()
        self.nodes_by_ck.clear()
        self.node_ids_by_parent_id.clear()

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
            if node.parent_ptr.id not in self.node_ids_by_parent_id:
                self.node_ids_by_parent_id[node.parent_ptr.id] = []
            self.node_ids_by_parent_id[node.parent_ptr.id].append(node.id)

    def update(self, node: NodeDataT):
        assert isinstance(node.id, str), f"cannot add {node!r} to {self!r} without id"
        existing = self.nodes_by_id.get(node.id)
        if existing is None:
            raise ValueError(f"node {node!r} does not exist in {self!r}")
        self.nodes_by_id[node.id] = node
        if hasattr(node, "ck"):
            self.nodes_by_ck[node.ck] = node
        if existing.parent_ptr is not None:
            self.node_ids_by_parent_id[existing.parent_ptr.id].remove(existing.id)
        if node.parent_ptr is not None:
            if node.parent_ptr.id not in self.node_ids_by_parent_id:
                self.node_ids_by_parent_id[node.parent_ptr.id] = []
            if node.id not in self.node_ids_by_parent_id[node.parent_ptr.id]:
                self.node_ids_by_parent_id[node.parent_ptr.id].append(node.id)

    def remove(self, node: NodeDataT):
        assert isinstance(node.id, str), f"cannot add {node!r} to {self!r} without id"
        if node.parent_ptr is not None:
            self.node_ids_by_parent_id[node.parent_ptr.id].remove(node.id)
        queue = deque([node])
        while queue:
            node = queue.popleft()
            self.nodes_by_id.pop(node.id, None)
            if hasattr(node, "ck"):
                self.nodes_by_ck.pop(node.ck, None)
            child_ids = self.node_ids_by_parent_id.pop(node.id, ())
            if len(child_ids) > 0:
                queue.extend(self.nodes_by_id[child_id] for child_id in child_ids)

    def find_roots(self) -> tuple[NodeDataT, ...]:
        return tuple(
            node
            for node in self.nodes_by_id.values()
            if node.parent_ptr is None or node.parent_ptr.id not in self.nodes_by_id
        )

    def collect_descendants(
        self, node: NodeDataT, node_type: NodeType | None = None, recursive: bool = False
    ) -> tuple["NodeDataT", ...] | list["NodeDataT"]:
        """Gets all children descendants as filtered in BFS order"""
        assert isinstance(node.id, str), f"expected NodeData, got {Node!r}"
        if not recursive:
            if node_type is not None:
                return tuple(
                    self.nodes_by_id[child_id]
                    for child_id in self.node_ids_by_parent_id.get(node.id, [])
                    if self.nodes_by_id[child_id].metatype == node_type
                )
            else:
                return tuple(
                    self.nodes_by_id[child_id]
                    for child_id in self.node_ids_by_parent_id.get(node.id, [])
                )
        else:
            descendants: list[NodeDataT] = []
            queue = deque(
                self.nodes_by_id[child_id]
                for child_id in self.node_ids_by_parent_id.get(node.id, [])
            )
            while queue:
                current_node = queue.popleft()
                descendants.append(current_node)
                for child_id in self.node_ids_by_parent_id.get(current_node.id, []):
                    queue.append(self.nodes_by_id[child_id])
            return descendants

    def walk_bfs(self, roots: list[NodeDataT] = None) -> Generator[NodeDataT, None, None]:
        """Walks the tree in breadth-first order"""
        if roots is not None and len(roots) == 0:
            return
        queue = deque(roots or self.find_roots())
        while queue:
            current_node = queue.popleft()
            yield current_node
            for child_id in self.node_ids_by_parent_id.get(current_node.id, []):
                queue.append(self.nodes_by_id[child_id])


class DetachedNodeTree(NodeTreeBase[NodeT, UUID]):
    """
    A NodeTree for nodes that may not have ids yet (are 'detached').
    We have a separate tree for this because wire nodes work with ids only (for parent),
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
        """Clear the tree"""
        self.nodes_by_ck.clear()
        self.nodes_by_parent_ck.clear()

    def add(self, node: "Node"):
        """Add a node to the tree (error if node already exists)"""
        if node.ck in self.nodes_by_ck and self.nodes_by_ck[node.ck] is not node:
            raise ValueError(f"node {node!r} (ck={node.ck}) already exists in {self!r}")
        self.nodes_by_ck[node.ck] = node

        if node.parent is not None:
            self.nodes_by_parent_ck[node.parent.ck].append(node)

    def update(self, node: "Node"):
        """Updates the node in this tree (must exist)"""
        existing = self.nodes_by_ck.get(node.ck)
        if existing is None:
            raise ValueError(f"node {node!r} (ck={node.ck}) does not exist in {self!r}")
        self.nodes_by_ck[node.ck] = node

        if existing.parent is not None and existing.parent in self.nodes_by_parent_ck:
            self.nodes_by_parent_ck[existing.parent_id].remove(existing)
        if node.parent is not None and node not in self.nodes_by_parent_ck[node.parent.ck]:
            self.nodes_by_parent_ck[node.parent.ck].append(node)

    def remove(self, node: "Node"):
        """Remove a node from the tree (incl. all descendants if recursive)"""
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
        node_type: NodeType | None = None,
        recursive: bool = False,
    ) -> tuple["NodeT", ...] | list["NodeT"]:
        """Gets all children descendants as filtered in BFS order"""
        assert isinstance(node.ck, UUID), f"expected UUID in node, got {node!r}"
        if not node.__has_scope__:
            return ()
        node_ck = node.ck
        if not recursive:
            if node_type is not None:
                return tuple(
                    child
                    for child in self.nodes_by_parent_ck.get(node_ck, ())
                    if child.metatype == node_type
                )
            else:
                return tuple(self.nodes_by_parent_ck.get(node_ck, ()))
        else:
            descendants: list[NodeT] = []
            queue = deque(self.nodes_by_parent_ck.get(node_ck, ()))
            while queue:
                current_node = queue.popleft()
                descendants.append(current_node)
                children = self.nodes_by_parent_ck.get(current_node.ck, ())
                if len(children) > 0:
                    queue.extend(children)
            return descendants
