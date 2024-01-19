import abc
from collections import defaultdict, deque
from typing import (
    TYPE_CHECKING,
    Collection,
    Generator,
    Generic,
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
        """Add a node to the tree (error if node already exists)"""
        raise NotImplementedError

    def update(self, node: "SomeNodeT"):
        """Updates an existing node in this tree (must exist)"""
        raise NotImplementedError

    def remove(self, node: "SomeNodeT"):
        """Remove a node from the tree (incl. all descendants)"""
        raise NotImplementedError

    def get_descendants(
        self,
        node_id: IdT,
        node_type: NodeType | None = None,
        recursive: bool = False,
        prefilter: bool = False,
        include_self: bool = False,
    ) -> list["NodeT"]:
        """Gets all descendants as filtered in BFS order"""
        raise NotImplementedError

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
    """An indexed tree of module nodes (UUIDs for ids, parent_ids)."""

    def __init__(self, nodes: Collection[NodeT] | "NodeTree" = None):
        self.nodes_by_id: dict[UUID, NodeT] = {}
        self.nodes_by_ck: dict[UUID, NodeT] = {}  # most nodes have a ck as well
        self.child_ids_by_parent_id: dict[UUID, list[UUID]] = {}
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
        self.child_ids_by_parent_id.clear()

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
            if node.parent_id not in self.child_ids_by_parent_id:
                self.child_ids_by_parent_id[node.parent_id] = []
            self.child_ids_by_parent_id[node.parent_id].append(node.id)

    def update(self, node: NodeT):
        assert isinstance(node.id, UUID), f"cannot add {node!r} to {self!r} without id"
        existing = self.nodes_by_id.get(node.id)
        if existing is None:
            raise ValueError(f"node {node!r} does not exist in {self!r}")
        self.nodes_by_id[node.id] = node
        self.nodes_by_ck[node.ck] = node
        if existing.parent_id is not None:
            self.child_ids_by_parent_id[existing.parent_id].remove(existing.id)
        if node.parent_id is not None:
            if node.parent_id not in self.child_ids_by_parent_id:
                self.child_ids_by_parent_id[node.parent_id] = []
            if node.id not in self.child_ids_by_parent_id[node.parent_id]:
                self.child_ids_by_parent_id[node.parent_id].append(node.id)

    def remove(self, node: NodeT):
        assert isinstance(node.id, UUID), f"cannot add {node!r} to {self!r} without id"
        descendants = self.get_descendants(node.id, recursive=True, include_self=True)
        for descendant in descendants:
            if descendant.id in self.nodes_by_id:
                self.nodes_by_id.pop(descendant.id)
            if descendant.ck in self.nodes_by_ck:
                self.nodes_by_ck.pop(descendant.ck)
            if descendant.id in self.child_ids_by_parent_id:
                self.child_ids_by_parent_id.pop(descendant.id)
            if descendant.parent_id in self.child_ids_by_parent_id:
                self.child_ids_by_parent_id[descendant.parent_id].remove(descendant.id)

    def get_descendants(
        self,
        node_id_or_ck: UUID,
        node_type: NodeType | None = None,
        recursive: bool = False,
        prefilter: bool = False,
        include_self: bool = False,
    ) -> list["NodeT"]:
        """Gets all children descendants as filtered in BFS order"""
        assert isinstance(node_id_or_ck, UUID), f"expected UUID, got {node_id_or_ck!r}"
        if node_id_or_ck in self.nodes_by_id:
            node_id_or_ck = node_id_or_ck
        elif node_id_or_ck in self.nodes_by_ck:
            node_id_or_ck = self.nodes_by_ck[node_id_or_ck].id
        else:
            raise ValueError(f"node {node_id_or_ck} is not in {self!r}")
        children = [
            self.nodes_by_id[child_id]
            for child_id in self.child_ids_by_parent_id.get(node_id_or_ck, [])
            if not node_type or not prefilter or self.nodes_by_id[child_id].metatype == node_type
        ]

        descendants = []
        if include_self:
            descendants.append(self.nodes_by_id[node_id_or_ck])
        descendants.extend(children)
        if recursive:
            for child in children:
                if child.id not in self.child_ids_by_parent_id:
                    continue
                descendants.extend(
                    self.get_descendants(child.id, node_type, prefilter=prefilter, recursive=True)
                )
        if not prefilter and node_type:
            descendants = [n for n in descendants if n.metatype == node_type]
        return descendants


class NodeDataTree(NodeTreeBase[NodeDataT, str]):
    """A NodeTree for NodeData objects (strings for ids, parent_ptr)."""

    def __init__(self, nodes: Collection[NodeDataT] | "NodeDataTree" = None):
        self.nodes_by_id: dict[str, NodeDataT] = {}
        self.nodes_by_ck: dict[str, NodeDataT] = {}  # *most* nodes have a ck as well
        self.child_ids_by_parent_id: dict[str, list[str]] = {}
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
        self.child_ids_by_parent_id.clear()

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
            if node.parent_ptr.id not in self.child_ids_by_parent_id:
                self.child_ids_by_parent_id[node.parent_ptr.id] = []
            self.child_ids_by_parent_id[node.parent_ptr.id].append(node.id)

    def update(self, node: NodeDataT):
        assert isinstance(node.id, str), f"cannot add {node!r} to {self!r} without id"
        existing = self.nodes_by_id.get(node.id)
        if existing is None:
            raise ValueError(f"node {node!r} does not exist in {self!r}")
        self.nodes_by_id[node.id] = node
        if hasattr(node, "ck"):
            self.nodes_by_ck[node.ck] = node
        if existing.parent_ptr is not None:
            self.child_ids_by_parent_id[existing.parent_ptr.id].remove(existing.id)
        if node.parent_ptr is not None:
            if node.parent_ptr.id not in self.child_ids_by_parent_id:
                self.child_ids_by_parent_id[node.parent_ptr.id] = []
            if node.id not in self.child_ids_by_parent_id[node.parent_ptr.id]:
                self.child_ids_by_parent_id[node.parent_ptr.id].append(node.id)

    def remove(self, node: NodeDataT):
        assert isinstance(node.id, str), f"cannot add {node!r} to {self!r} without id"
        descendants = self.get_descendants(node.id, recursive=True, include_self=True)
        for n in descendants:
            if n.id in self.nodes_by_id:
                self.nodes_by_id.pop(n.id)
            if getattr(n, "ck", None) in self.nodes_by_ck:
                self.nodes_by_ck.pop(n.ck)
            if n.id in self.child_ids_by_parent_id:
                self.child_ids_by_parent_id.pop(n.id)
            if n.parent_ptr is not None and n.parent_ptr.id in self.child_ids_by_parent_id:
                self.child_ids_by_parent_id[n.parent_ptr.id].remove(n.id)

    def get_descendants(
        self,
        node_id_or_ck: str,
        node_type: NodeType | None = None,
        recursive: bool = False,
        prefilter: bool = False,
        include_self: bool = False,
    ) -> list["NodeDataT"]:
        """Gets all children descendants as filtered in BFS order"""
        assert isinstance(node_id_or_ck, str), f"expected str, got {node_id_or_ck!r}"
        if node_id_or_ck in self.nodes_by_id:
            node_id_or_ck = node_id_or_ck
        elif node_id_or_ck in self.nodes_by_ck:
            node_id_or_ck = self.nodes_by_ck[node_id_or_ck].id
        else:
            raise ValueError(f"node {node_id_or_ck} is not in {self!r}")
        children = [
            self.nodes_by_id[child_id]
            for child_id in self.child_ids_by_parent_id.get(node_id_or_ck, [])
            if not node_type or not prefilter or self.nodes_by_id[child_id].metatype == node_type
        ]

        descendants = []
        if include_self:
            descendants.append(self.nodes_by_id[node_id_or_ck])
        descendants.extend(children)
        if recursive:
            for child in children:
                if child.id not in self.child_ids_by_parent_id:
                    continue
                descendants.extend(
                    self.get_descendants(child.id, node_type, prefilter=prefilter, recursive=True)
                )
        if not prefilter and node_type:
            descendants = [n for n in descendants if n.metatype == node_type]
        return descendants

    @property
    def roots(self) -> list[NodeDataT]:
        return [
            node
            for node in self.nodes_by_id.values()
            if node.parent_ptr is None or node.parent_ptr.id not in self.nodes_by_id
        ]

    @property
    def root(self) -> Optional[NodeDataT]:
        roots = self.roots
        if len(roots) > 1:
            raise ValueError(f"expected 0 or 1 root nodes, got {roots}")
        return roots[0] if roots else None

    def walk_bfs(self, roots: list[NodeDataT] = None) -> Generator[NodeDataT, None, None]:
        """Walks the tree in breadth-first order"""
        num_traversed = 0
        queue = deque(roots or self.roots)
        while queue:
            current_node = queue.popleft()
            num_traversed += 1
            yield current_node
            for child_id in self.child_ids_by_parent_id.get(current_node.id, []):
                queue.append(self.nodes_by_id[child_id])
        if roots == self.roots and num_traversed != len(self.nodes_by_id):
            raise ValueError(
                f"expected {len(self.nodes_by_id)} nodes, but traversed {num_traversed}"
            )


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
        node_ck = node.ck
        node_parent_ck = node.parent.ck
        existing = self.nodes_by_ck.get(node_ck)
        if existing is None:
            raise ValueError(f"node {node!r} (ck={node_ck}) does not exist in {self!r}")
        self.nodes_by_ck[node_ck] = node
        if existing.parent is not None and existing.parent in self.nodes_by_parent_ck:
            self.nodes_by_parent_ck[existing.parent_id].remove(existing)
        if node.parent is not None and node not in self.nodes_by_parent_ck[node_parent_ck]:
            self.nodes_by_parent_ck[node_parent_ck].append(node)

    def remove(self, node: "Node"):
        """Remove a node from the tree (incl. all descendants if recursive)"""
        descendants = self.get_descendants(node.ck, recursive=True, include_self=True)
        for descendant in descendants:
            descendant_ck = descendant.ck
            if descendant_ck in self.nodes_by_parent_ck:
                self.nodes_by_parent_ck.pop(descendant_ck)
            if descendant.parent and descendant.parent.ck in self.nodes_by_parent_ck:
                self.nodes_by_parent_ck[descendant.parent.ck].remove(descendant)
            if descendant_ck in self.nodes_by_ck:
                self.nodes_by_ck.pop(descendant_ck)

    def get_descendants(
        self,
        node_id_or_ck: UUID,
        node_type: NodeType | None = None,
        recursive: bool = False,
        prefilter: bool = False,
        include_self: bool = False,
    ) -> list["NodeT"]:
        """Gets all children descendants as filtered in BFS order"""
        assert isinstance(node_id_or_ck, UUID), f"expected UUID, got {node_id_or_ck!r}"
        children = [
            child
            for child in self.nodes_by_parent_ck.get(node_id_or_ck, [])
            if not node_type or not prefilter or child.metatype == node_type
        ]
        descendants = []
        if include_self:
            descendants.append(self.nodes_by_ck[node_id_or_ck])
        descendants.extend(children)
        if recursive:
            for child in children:
                if child.ck not in self.nodes_by_parent_ck:
                    continue
                descendants.extend(
                    self.get_descendants(child.ck, node_type, prefilter=prefilter, recursive=True)
                )
        if not prefilter and node_type:
            descendants = [n for n in descendants if n.metatype == node_type]
        return descendants
