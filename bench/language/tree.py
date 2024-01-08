import abc
from collections import defaultdict, deque
import dataclasses
from datetime import datetime
from typing import (
    TYPE_CHECKING,
    Collection,
    Generator,
    Generic,
    Iterator,
    Optional,
    TypeVar,
    Union,
)
from uuid import UUID

from bench.language.const import EditKind, NodeType, to_bench_metatype, INTERP_NODE_TYPES
from bench.proto.wire import EditData, AnyNodeData
from bench.utils.func import to_uuid
from bench.utils.utils import flatten

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

    def add(self, node: "NT"):
        """Add a node to the tree (error if node already exists)"""
        raise NotImplementedError

    def add_many(self, *nodes: Collection[NT]):
        nodes = flatten(*nodes)
        for node in nodes:
            self.add(node)

    def update(self, node: "NT"):
        """Updates the node in this tree (must exist) nocheckin: handle move (and other?) updates"""
        raise NotImplementedError

    def set(self, nodes: Collection[NT]):
        """Replaces all nodes in the tree"""
        self.clear()
        for node in nodes:
            self.add(node)

    def add_tree(self, tree: "DetachedNodeTree"):
        raise NotImplementedError

    def remove(self, node: "NT"):
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

    def collect_descendants(self, nodes: Collection[NT]) -> list["NT"]:
        """Gets all descendants in BFS order"""
        raise NotImplementedError

    def get_ancestor(self, node_id: UUID, node_type: NodeType | None = None) -> Optional["NT"]:
        """Finds the next ancestor of the given type (including self)"""
        raise NotImplementedError

    def ancestor(self, node_id: UUID, node_type: NodeType | None = None) -> "NT":
        """Gets the next ancestor of the given type (including self)"""
        node = self.get_ancestor(node_id, node_type)
        if node is None:
            raise LookupError(f"no ancestor of type {node_type} for node {node_id!r}")
        return node

    def apply_edit(self, edit: EditData):
        """Applies a list of edits to the tree"""
        if edit.kind in (EditKind.CREATE, EditKind.RESTORE):
            self.add(edit.node)
        elif edit.kind in (EditKind.UPDATE, EditKind.MOVE):  # move not yet supported
            self.update(edit.node)
        elif edit.kind in (EditKind.DELETE, EditKind.SOFT_DELETE):
            self.remove(edit.node)
        else:
            raise ValueError(f"unexpected edit: {edit!r}")


class NodeTree(NodeTreeBase[NT]):
    """An indexed tree of module nodes. Can be either language or data nodes."""

    def __init__(self, nodes: Collection[NT] | "NodeTree" = None):
        self.nodes_by_id: dict[UUID, NT] = {}
        self.nodes_by_ck: dict[UUID, NT] = {}
        self.node_id_by_parent_id: dict[UUID, list[UUID]] = {}
        if isinstance(nodes, list):
            for node in nodes or []:
                self.add(node)
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

    def deepcopy(self):
        from bench.proto.wiring import copy_struct_data

        nodes = [copy_struct_data(node) for node in self.nodes]
        return NodeTree(nodes)

    #
    # Edits
    #

    def clear(self):
        """Clear the tree"""
        self.nodes_by_id.clear()
        self.nodes_by_ck.clear()
        self.node_id_by_parent_id.clear()

    def add(self, node: NT):
        """Add a node to the tree (error if node already exists)"""
        assert node.id is not None, f"cannot add {node!r} to {self!r} without id"
        node_id = to_uuid(node.id)
        if node_id in self.nodes_by_id:
            existing = self.nodes_by_id[node_id]
            raise ValueError(
                f"node {node!r} (id={node_id}) already exists in {self!r}: {existing!r} (id={existing.id})"
            )
        self.nodes_by_id[node_id] = node
        self.nodes_by_ck[to_uuid(node.ck)] = node
        parent_id = to_uuid(node.parent_id)
        if parent_id is not None:
            if parent_id not in self.node_id_by_parent_id:
                self.node_id_by_parent_id[parent_id] = []
            self.node_id_by_parent_id[parent_id].append(node_id)

    def update(self, node: NT):
        """Updates a node in this tree (must exist)"""
        node_id = to_uuid(node.id)
        existing = self.nodes_by_id.get(node_id)
        if existing is None:
            raise ValueError(f"node {node!r} does not exist in {self!r}")
        self.nodes_by_id[node_id] = node
        self.nodes_by_ck[to_uuid(node.ck)] = node
        if existing.parent_id is not None:
            self.node_id_by_parent_id[existing.parent_id].remove(existing.id)
        parent_id = to_uuid(node.parent_id)
        if parent_id is not None:
            if parent_id not in self.node_id_by_parent_id:
                self.node_id_by_parent_id[parent_id] = []
            if node_id not in self.node_id_by_parent_id[parent_id]:
                self.node_id_by_parent_id[parent_id].append(node_id)

    def replace(self, node: NT):
        """Upsert a node in the tree (replace if node already exists)"""
        node_id = to_uuid(node.id)
        old_node = self.nodes_by_id.get(node_id)
        if old_node is not None and old_node.parent_id is not None:
            self.node_id_by_parent_id[old_node.parent_id].remove(node_id)
        self.update(node)

    def remove(self, node: NT):
        """Remove a node from the tree (incl. all descendants if recursive)"""
        descendants = self.get_descendants(node.id, recursive=True, include_self=True)
        for descendant in descendants:
            descendant_id = to_uuid(descendant.id)
            descendant_ck = to_uuid(descendant.ck)
            if descendant_id in self.nodes_by_id:
                self.nodes_by_id.pop(descendant_id)
            if descendant_ck in self.nodes_by_ck:
                self.nodes_by_ck.pop(descendant_ck)
            if descendant_id in self.node_id_by_parent_id:
                self.node_id_by_parent_id.pop(descendant_id)
            descendant_parent_id = to_uuid(descendant.parent_id)
            if descendant_parent_id in self.node_id_by_parent_id:
                self.node_id_by_parent_id[descendant_parent_id].remove(descendant_id)

    def prune(self, t: type[NT]):
        """Prune all nodes of the given type"""
        for node in list(self.nodes_by_id.values()):
            if isinstance(node, t):
                self.remove(node, recursive=True)

    def add_tree(self, tree: Union["NodeTree", "DetachedNodeTree"]):
        if isinstance(tree, DetachedNodeTree):
            for node in tree.nodes_by_ck.values():
                if node.metatype != NodeType.RECORD:  # remove hoisted records :TempRecordTree
                    self.add(node)
        else:
            self.nodes_by_id.update(tree.nodes_by_id)
            self.nodes_by_ck.update(tree.nodes_by_ck)
            self.node_id_by_parent_id.update(tree.node_id_by_parent_id)

    def remove_tree(self, tree: "NodeTree"):
        for node in tree.nodes_by_id.values():
            node_id = to_uuid(node.id)
            node_ck = to_uuid(node.ck)
            if node_id in self.nodes_by_id:
                self.nodes_by_id.pop(node_id)
            if node_ck in self.nodes_by_ck:
                self.nodes_by_ck.pop(node_ck)
            if node_id in self.node_id_by_parent_id:
                self.node_id_by_parent_id.pop(node_id)

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

    def path_of(self, node: NT) -> list[NT]:
        """Returns the path from the root to the node"""
        path = []
        while node:
            path.insert(0, node)
            node = self.nodes_by_id.get(to_uuid(node.parent_id))
        return path

    @property
    def roots(self) -> list[NT]:
        return [
            node
            for node in self.nodes_by_id.values()
            if node.parent_id is None or to_uuid(node.parent_id) not in self.nodes_by_id
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
            for child_id in self.node_id_by_parent_id.get(to_uuid(current_node.id), []):
                queue.append(self.nodes_by_id[child_id])
        if roots == self.roots and num_traversed != len(self.nodes_by_id):
            raise ValueError(
                f"expected {len(self.nodes_by_id)} nodes, but traversed {num_traversed}"
            )

    def walk_bfs_batched(self, roots: list[NT] = None) -> Generator[list[NT], None, None]:
        """Walks the tree in breadth-first order, yielding all nodes at each level"""
        num_traversed = 0
        queue = deque(roots or self.roots)
        while queue:
            level = []
            for _ in range(len(queue)):
                current_node = queue.popleft()
                level.append(current_node)
                for child_id in self.node_id_by_parent_id.get(current_node.id, []):
                    queue.append(self.nodes_by_id[child_id])
            num_traversed += len(level)
            yield level
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
            for child_id in self.node_id_by_parent_id.get(node_id, [])
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
                if child.id not in self.node_id_by_parent_id:
                    continue
                descendants.extend(
                    self.get_descendants(child.id, node_type, prefilter=prefilter, recursive=True)
                )
        if not prefilter and node_type:
            descendants = [n for n in descendants if to_bench_metatype(n.metatype) == node_type]
        return descendants

    def collect_descendants(self, nodes: Collection[NT]) -> Collection["NT"]:
        """Gets all descendants in BFS order"""
        descendants_by_ck: dict[UUID, NT] = {}
        children = deque(nodes)
        while children:
            child = children.popleft()
            if child.ck not in descendants_by_ck:
                descendants_by_ck[child.ck] = child
                if child.id in self.node_id_by_parent_id:
                    children.extend(
                        self.nodes_by_id[n] for n in self.node_id_by_parent_id[child.id]
                    )
        return descendants_by_ck.values()

    def get_ancestor(self, node_id: UUID, node_type: NodeType | None = None) -> Optional["NT"]:
        """Finds the next ancestor of the given type (including self)"""
        assert isinstance(node_id, UUID), f"expected UUID, got {node_id!r}"
        if node_id in self.nodes_by_id:
            node_id = node_id
        elif node_id in self.nodes_by_ck:
            node_id = self.nodes_by_ck[node_id].id
        else:
            raise ValueError(f"node {node_id} is not in {self!r}")
        node = self.nodes_by_id.get(node_id)
        while node:
            if not node_type or to_bench_metatype(node.metatype) == node_type:
                return node
            if node.parent_id is None:
                return None
            node = self.nodes_by_id[to_uuid(node.parent_id)]
        return None

    def get_ancestors(
        self,
        node_id: UUID,
        node_type: NodeType | None = None,
        include_self: bool = False,
    ) -> list["NT"]:
        """Finds all ancestors of the given type"""
        assert isinstance(node_id, UUID), f"expected UUID, got {node_id!r}"
        ancestors = []
        node = self.nodes_by_id.get(node_id)
        if node is None:
            raise ValueError(f"node {node_id} is not in {self!r}")
        if include_self:
            ancestors.append(node)
        while node:
            if not node_type or to_bench_metatype(node.metatype) == node_type:
                ancestors.append(node)
            if node.parent_id is None:
                break
            node = self.nodes_by_id[to_uuid(node.parent_id)]
        return ancestors


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

    def add(self, node: "Node"):
        """Add a node to the tree (error if node already exists)"""
        node_ck = to_uuid(node.ck)
        if node_ck in self.nodes_by_ck and self.nodes_by_ck[node_ck] is not node:
            raise ValueError(f"node {node!r} (ck={node_ck}) already exists in {self!r}")
        self.nodes_by_ck[node_ck] = node
        if node.parent is not None:
            self.nodes_by_parent_ck[node.parent.ck].append(node)

    def update(self, node: "Node"):
        """Updates the node in this tree (must exist)"""
        node_ck = to_uuid(node.ck)
        node_parent_ck = to_uuid(node.parent.ck)
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

    def remove(self, node: "Node"):
        """Remove a node from the tree (incl. all descendants if recursive)"""
        descendants = self.get_descendants(to_uuid(node.ck), recursive=True, include_self=True)
        for descendant in descendants:
            descendant_ck = to_uuid(descendant.ck)
            if descendant_ck in self.nodes_by_parent_ck:
                self.nodes_by_parent_ck.pop(descendant_ck)
            if descendant.parent and to_uuid(descendant.parent.ck) in self.nodes_by_parent_ck:
                self.nodes_by_parent_ck[to_uuid(descendant.parent.ck)].remove(descendant)
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
                child_ck = to_uuid(child.ck)
                if child_ck not in self.nodes_by_parent_ck:
                    continue
                descendants.extend(
                    self.get_descendants(child_ck, node_type, prefilter=prefilter, recursive=True)
                )
        if not prefilter and node_type:
            descendants = [n for n in descendants if n.metatype == node_type]
        return descendants

    def collect_descendants(self, nodes: Collection[NT]) -> Collection["NT"]:
        """Gets all descendants in BFS order"""
        descendants_by_ck: dict[UUID, NT] = {}
        children = deque(nodes)
        while children:
            child = children.popleft()
            child_ck = to_uuid(child.ck)
            if child_ck not in descendants_by_ck:
                descendants_by_ck[child_ck] = child
                if child_ck in self.nodes_by_parent_ck:
                    children.extend(self.nodes_by_parent_ck[child_ck])
        return descendants_by_ck.values()

    def get_ancestor(self, node_id: UUID, node_type: NodeType | None = None) -> Optional["NT"]:
        """Finds the next ancestor of the given type (including self)"""
        assert isinstance(node_id, UUID), f"expected UUID, got {node_id!r}"
        node = self.nodes_by_ck.get(node_id)
        if node is None:
            raise ValueError(f"node {node_id} is not in {self!r}")
        while node:
            if not node_type or node.metatype == node_type:
                return node
            if node.parent is None:
                return None
            node = node.parent
        return None


# NOTE: we don't generate EditData yet because we don't have nodes as 'full' inline properties
#  EditData is defined manually in our extra proto file.
# @struct(StructType.EDIT)
# class Edit:
#     """
#     An edit to a module/node.
#     """
#
#     kind: EditKind = struct_internal(20)
#     module: "Module" = struct_internal(21, references=NodeType.MODULE)
#     node: "Node" = --> <???> <--
#     revision: Optional[int] = struct_internal(23, default=None)
#     properties: list[str] = struct_internal(24, default=None)


EditableNode = Union["Node", AnyNodeData]


class NodeTreeEditor:
    """Create edits to a module node tree."""

    def __init__(
        self,
        tree: "NodeTree",
        bench_id: UUID,
        module_id: UUID,
        # default file and statement id
        file_id: UUID = None,
        statement_id: UUID = None,
    ):
        self.tree = tree
        self.bench_id = bench_id
        self.module_id = module_id
        self.file_id = file_id
        self.statement_id = statement_id
        self.edits = []

    def __str__(self):
        return f"edit {len(self.edits)} {self.module_id}"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}>"

    def reset(self):
        self.edits = []

    def _make_edit(
        self,
        kind: EditKind,
        node: EditableNode,
        properties: list[str] = None,
        scope: NodeType = None,
    ) -> EditData:
        from bench.proto.wiring import wrap_some_node

        edit = EditData(
            kind=kind,
            module_id=self.module_id,
            scope=scope,
            properties=properties,
            node=wrap_some_node(self._pack_node_flat_if_needed(node)),
        )
        self.edits.append(edit)
        return edit

    def _pack_node_flat_if_needed(self, node: Union["Node", EditableNode]) -> "AnyNodeData":
        from bench.proto import wiring

        if isinstance(node, Node):
            return wiring.pack_node(node)
        else:
            return dataclasses.replace(node)  # shallow copy

    def create_many(self, *nodes: EditableNode) -> list[EditData]:
        return [self.create(node) for node in nodes]

    def create(self, node: EditableNode) -> EditData:
        return self._make_edit(EditKind.CREATE, node)

    def update(self, node: EditableNode, properties: list[str] = None) -> EditData:
        assert isinstance(properties, list) or properties is None, f"invalid props: {properties}"
        return self._make_edit(EditKind.UPDATE, node, properties)

    def move(self, node: EditableNode) -> EditData:
        return self._make_edit(EditKind.MOVE, node)

    def soft_delete_many(self, *nodes: EditableNode) -> list[EditData]:
        return [self.soft_delete(node) for node in nodes]

    def soft_delete(self, node: EditableNode) -> EditData:
        # sneakily convert soft delete into hard delete for interp types
        if node.metatype in INTERP_NODE_TYPES:
            return self.delete(node)
        return self._make_edit(EditKind.SOFT_DELETE, self._pack_node_flat_if_needed(node))

    def restore(self, node: EditableNode) -> EditData:
        return self._make_edit(EditKind.RESTORE, self._pack_node_flat_if_needed(node))

    def archive(self, node: EditableNode) -> EditData:
        return self._make_edit(EditKind.ARCHIVE, self._pack_node_flat_if_needed(node))

    def unarchive(self, node: EditableNode) -> EditData:
        return self._make_edit(EditKind.UNARCHIVE, self._pack_node_flat_if_needed(node))

    def delete(self, node: EditableNode) -> EditData:
        return self._make_edit(EditKind.DELETE, node)
