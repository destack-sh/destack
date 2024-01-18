import abc
from collections import defaultdict, deque
from typing import (
    TYPE_CHECKING,
    Collection,
    Generator,
    Generic,
    Iterator,
    Optional,
    TypeVar,
)
from uuid import UUID

from bench.language.const import EditKind, NodeType, to_bench_metatype
from bench.proto.wire import EditData

if TYPE_CHECKING:
    from bench.language import Node

NT = TypeVar("NT")


def walk_bfs(nodes: Collection[NT]) -> Iterator[NT]:
    """Walks nodes in BFS order."""
    node_ids = {n.id for n in nodes}
    nodes_by_parent_id = defaultdict(list)
    for node in nodes:
        nodes_by_parent_id[node.parent_id].append(node)

    queue = deque(n for n in nodes if n.parent_id not in node_ids)
    while queue:
        node = queue.popleft()
        yield node
        queue.extend(nodes_by_parent_id[node.id])


class NodeTreeBase(abc.ABC, Generic[NT]):
    @property
    def nodes(self) -> Collection[NT]:
        raise NotImplementedError

    def get(self, node_id: UUID) -> Optional[NT]:
        """Gets a node by id or ck"""
        raise NotImplementedError

    def __getitem__(self, item):
        raise NotImplementedError

    def __contains__(self, item):
        raise NotImplementedError

    def clear(self):
        """Clear the tree"""
        raise NotImplementedError

    def create(self, node: "NT"):
        """Add a node to the tree (error if node already exists)"""
        raise NotImplementedError

    def upsert(self, node: "NT"):
        """Add a node to the tree (error if node already exists)"""
        pass

    def update(self, node: "NT"):
        """Updates the node in this tree (must exist)"""
        raise NotImplementedError

    def set(self, nodes: Collection[NT]):
        """Replaces all nodes in the tree"""
        self.clear()
        for node in nodes:
            self.create(node)

    def add_tree(self, tree: "DetachedNodeTree"):
        for node in tree.nodes:
            self.create(node)

    def delete(self, node: "NT"):
        """Remove a node from the tree (incl. all descendants if recursive)"""
        raise NotImplementedError

    def get_descendants(
        self,
        node_id: UUID,
        node_type: NodeType | None = None,
        recursive: bool = False,
        prefilter: bool = False,
        include_self: bool = False,
    ) -> list["NT"]:
        """Gets all descendants as filtered in BFS order"""
        raise NotImplementedError

    def apply_edit(self, edit: EditData):
        """Applies a list of edits to the tree"""
        if edit.kind in (EditKind.CREATE, EditKind.RESTORE, EditKind.UNARCHIVE):
            self.create(edit.node)
        elif edit.kind in (EditKind.UPDATE, EditKind.MOVE):  # move not yet supported
            self.update(edit.node)
        elif edit.kind in (EditKind.DELETE, EditKind.SOFT_DELETE, EditKind.ARCHIVE):
            self.delete(edit.node)
        else:
            raise ValueError(f"unexpected edit: {edit!r}")


class NodeTree(NodeTreeBase[NT]):
    """An indexed tree of module nodes. Can be either language or data nodes."""

    def __init__(self, nodes: Collection[NT] | "NodeTree" = None):
        self.nodes_by_id: dict[UUID, NT] = {}
        self.nodes_by_ck: dict[UUID, NT] = {}
        self.child_ids_by_parent_id: dict[UUID, list[UUID]] = {}
        if isinstance(nodes, list):
            for node in nodes or []:
                self.create(node)
        elif isinstance(nodes, NodeTree):
            self.add_tree(nodes)

    def __str__(self):
        return f"{len(self.nodes_by_id)} nodes"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}>"

    @property
    def nodes(self) -> Collection[NT]:
        return self.nodes_by_ck.values()

    def copy(self):
        return NodeTree(self)

    #
    # Edits
    #

    def clear(self):
        """Clear the tree"""
        self.nodes_by_id.clear()
        self.nodes_by_ck.clear()
        self.child_ids_by_parent_id.clear()

    def create(self, node: NT):
        """Add a node to the tree (error if node already exists)"""
        from bench.language.node import Node

        assert isinstance(node, Node), f"expected Node, got {node!r}"
        assert node.id is not None, f"cannot add {node!r} to {self!r} without id"
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

    def update(self, node: NT):
        """Updates a node in this tree (must exist)"""
        from bench.language.node import Node

        assert isinstance(node, Node), f"expected Node, got {node!r}"
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

    def delete(self, node: NT):
        """Remove a node from the tree (incl. all descendants if recursive)"""
        from bench.language.node import Node

        assert isinstance(node, Node), f"expected Node, got {node!r}"
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

    #
    # Read only
    #

    def get(self, node_id: UUID) -> Optional[NT]:
        """Gets a node by id"""
        assert isinstance(node_id, UUID), f"expected UUID, got {node_id!r}"
        node = self.nodes_by_id.get(node_id)
        return node if node is not None else self.nodes_by_ck.get(node_id)

    def __getitem__(self, item):
        return self.get(item)

    def __contains__(self, item):
        return item in self.nodes_by_id or item in self.nodes_by_ck

    @property
    def roots(self) -> list[NT]:
        return [
            node
            for node in self.nodes_by_id.values()
            if node.parent_id is None or node.parent_id not in self.nodes_by_id
        ]

    @property
    def root(self) -> Optional[NT]:
        roots = self.roots
        if len(roots) > 1:
            raise ValueError(f"expected 0 or 1 root nodes, got {roots}")
        return roots[0] if roots else None

    def walk_bfs(self, roots: list[NT] = None) -> Generator[NT, None, None]:
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

    def get_descendants(
        self,
        node_id: UUID,
        node_type: NodeType | None = None,
        recursive: bool = False,
        prefilter: bool = False,
        include_self: bool = False,
    ) -> list["NT"]:
        """Gets all children descendants as filtered in BFS order"""
        assert isinstance(node_id, UUID), f"expected UUID, got {node_id!r}"
        if node_id in self.nodes_by_id:
            node_id = node_id
        elif node_id in self.nodes_by_ck:
            node_id = self.nodes_by_ck[node_id].id
        else:
            raise ValueError(f"node {node_id} is not in {self!r}")
        children = [
            self.nodes_by_id[child_id]
            for child_id in self.child_ids_by_parent_id.get(node_id, [])
            if not node_type
            or not prefilter
            or to_bench_metatype(self.nodes_by_id[child_id].metatype) == node_type
        ]

        descendants = []
        if include_self:
            descendants.append(self.nodes_by_id[node_id])
        descendants.extend(children)
        if recursive:
            for child in children:
                if child.id not in self.child_ids_by_parent_id:
                    continue
                descendants.extend(
                    self.get_descendants(child.id, node_type, prefilter=prefilter, recursive=True)
                )
        if not prefilter and node_type:
            descendants = [n for n in descendants if to_bench_metatype(n.metatype) == node_type]
        return descendants


class DetachedNodeTree(NodeTreeBase[NT]):
    """
    A minimal NodeTree for nodes that may not have ids yet (are 'detached' from a module).
    We have a separate tree for this because wire nodes work with ids only (for parent),
     and we don't need to support all operations since it's only for detached nodes.
    """

    def __init__(self):
        self.nodes_by_ck: dict[UUID, "Node"] = {}
        self.nodes_by_parent_ck: dict[UUID, list["Node"]] = defaultdict(list)

    def __str__(self):
        return f"{len(self.nodes_by_ck)} nodes"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}>"

    @property
    def nodes(self) -> Collection[NT]:
        return self.nodes_by_ck.values()

    def get(self, node_ck: UUID) -> Optional[NT]:
        """Gets a node by id"""
        assert isinstance(node_ck, UUID), f"expected UUID, got {node_ck!r}"
        return self.nodes_by_ck.get(node_ck)

    def __getitem__(self, item):
        return self.nodes_by_ck.get(item)

    def __contains__(self, item):
        return item in self.nodes_by_ck

    def clear(self):
        """Clear the tree"""
        self.nodes_by_ck.clear()
        self.nodes_by_parent_ck.clear()

    def create(self, node: "Node"):
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

    def add_tree(self, tree: "DetachedNodeTree"):
        assert type(self) == type(tree), f"cannot add {tree!r} to {self!r}"
        self.nodes_by_ck.update(tree.nodes_by_ck)
        for parent_ck, children in tree.nodes_by_parent_ck.items():
            self.nodes_by_parent_ck[parent_ck].extend(children)

    def delete(self, node: "Node"):
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
        node_id: UUID,
        node_type: NodeType | None = None,
        recursive: bool = False,
        prefilter: bool = False,
        include_self: bool = False,
    ) -> list["NT"]:
        """Gets all children descendants as filtered in BFS order"""
        assert isinstance(node_id, UUID), f"expected UUID, got {node_id!r}"
        children = [
            child
            for child in self.nodes_by_parent_ck.get(node_id, [])
            if not node_type or not prefilter or child.metatype == node_type
        ]
        descendants = []
        if include_self:
            descendants.append(self.nodes_by_ck[node_id])
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
