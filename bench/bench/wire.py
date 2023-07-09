from __future__ import annotations

import abc
import dataclasses
import re
import traceback
import typing
from collections import OrderedDict, deque
from dataclasses import dataclass, replace
from datetime import datetime
from typing import Any, ClassVar, Optional
from uuid import UUID

from bench import bench as lang
from bench.bench import StatementType
from bench.bench.const import (
    DatasetBackend,
    ExecutionTriggerType,
    RemoteObjectStatus,
    TypeFlag,
    TypeHint,
    TypeTag,
)
from bench.bench.core import (
    CRUD_PROPERTIES,
    MOT,
    InterpScope,
    ModuleNode,
    ModuleObjectType,
    Session,
)
from bench.bench.execution import ExecutionCodeFrame, ExecutionFrame, RunError, RunErrorKind
from bench.bench.issue import IssueKind, IssueType
from bench.bench.query import Query, Sort
from bench.utils.func import describe_type
from bench.utils.serialize import from_dict, to_dict

#
# Stable, concise and flat language data nodes for transit and storage.
# TODO @Performance @Robustness: use an optimized and evolvable :WireFormat
#


ParentsT = set[MOT]
NodeDataT = typing.TypeVar("NodeDataT", bound="NodeData")
NodeT = typing.TypeVar("NodeT", bound="ModuleNode")
DataT = typing.TypeVar("DataT")
ObjectT = typing.TypeVar("ObjectT")


class ModuleTree:
    """An indexed tree of module nodes"""

    def __init__(self, nodes: list[NodeT | NodeDataT] = None):
        self.nodes: dict[UUID, NodeT] = {}
        self.children: dict[UUID, list[UUID]] = {}
        for node in nodes or []:
            self.add(node)

    def __str__(self):
        return f"{len(self.nodes)} nodes"

    def __repr__(self):
        return f"<ModuleTree {self}>"

    def __contains__(self, item):
        return item in self.nodes

    def __getitem__(self, item):
        return self.nodes[item]

    def add(self, node: NodeT | NodeDataT):
        """Add a node to the tree (error if node already exists)"""
        if node.id in self.nodes:
            existing = self.nodes[node.id]
            raise ValueError(f"node {node} (id={node.id}) already exists in {self}: {existing}")
        self.nodes[node.id] = node
        if node.parent_id is not None:
            if node.parent_id not in self.children:
                self.children[node.parent_id] = []
            self.children[node.parent_id].append(node.id)

    def replace(self, node: NodeT | NodeDataT):
        """Upsert a node in the tree (replace if node already exists)"""
        old_node = self.nodes.get(node.id)
        if old_node is not None and old_node.parent_id is not None:
            self.children[old_node.parent_id].remove(node.id)
        self.nodes[node.id] = node
        if node.parent_id not in self.children:
            self.children[node.parent_id] = []
        self.children[node.parent_id].append(node.id)

    def remove(self, node: NodeT | NodeDataT):
        """Remove a node from the tree (incl. all descendants if recursive)"""
        descendants = self.get_descendants(node.id, recursive=True, include_self=True)
        for descendant in descendants:
            if descendant.id in self.nodes:
                self.nodes.pop(descendant.id)
            if descendant.id in self.children:
                self.children.pop(descendant.id)
            if descendant.parent_id in self.children:
                self.children[descendant.parent_id].remove(descendant.id)

    def truncate(
        self, node: NodeT | NodeDataT, t: NodeT | NodeDataT | None = None, recursive: bool = True
    ):
        """Truncate descendants of a node"""
        descendants = self.get_descendants(node.id, t, recursive=recursive)
        for descendant in descendants:
            if descendant.id in self.children:
                self.children.pop(descendant.id)
            if descendant.parent_id in self.children:
                self.children[descendant.parent_id].remove(descendant.id)
            self.nodes.pop(descendant.id)

    def prune(self, t: NodeT | NodeDataT):
        """Prune all nodes of the given type"""
        for node in list(self.nodes.values()):
            if isinstance(node, t):
                self.remove(node, recursive=True)

    @property
    def roots(self) -> list[NodeT | NodeDataT]:
        return [
            node
            for node in self.nodes.values()
            if node.parent_id is None or node.parent_id not in self.nodes
        ]

    @property
    def root(self) -> Optional[NodeT | NodeDataT]:
        roots = self.roots
        if len(roots) > 1:
            raise ValueError(f"expected 0 or 1 root nodes, got {roots}")
        return roots[0] if roots else None

    def walk_bfs(
        self, roots: list[NodeT | NodeDataT] = None
    ) -> typing.Generator[NodeT | NodeDataT, None, None]:
        """Walks the tree in breadth-first order"""
        num_traversed = 0
        queue = deque(roots or self.roots)
        while queue:
            current_node = queue.popleft()
            num_traversed += 1
            yield current_node
            for child_id in self.children.get(current_node.id, []):
                queue.append(self.nodes[child_id])
        if roots == self.roots and num_traversed != len(self.nodes):
            raise ValueError(f"expected {len(self.nodes)} nodes, but traversed {num_traversed}")

    def walk_bfs_batched(
        self, roots: list[NodeT | NodeDataT] = None
    ) -> typing.Generator[list[NodeT | NodeDataT], None, None]:
        """Walks the tree in breadth-first order, yielding all nodes at each level"""
        num_traversed = 0
        queue = deque(roots or self.roots)
        while queue:
            level = []
            for _ in range(len(queue)):
                current_node = queue.popleft()
                level.append(current_node)
                for child_id in self.children.get(current_node.id, []):
                    queue.append(self.nodes[child_id])
            num_traversed += len(level)
            yield level
        if roots == self.roots and num_traversed != len(self.nodes):
            raise ValueError(f"expected {len(self.nodes)} nodes, but traversed {num_traversed}")

    def get_child(
        self, parent_id: UUID, t: NodeT | NodeDataT | None = None
    ) -> Optional["NodeT | NodeDataT"]:
        """Finds one or zero children of the given type"""
        children = self.get_descendants(parent_id, t)
        if len(children) > 1:
            raise ValueError(
                f"expected 0 or 1 children of type {t} for parent {parent_id}, got {children}"
            )
        return children[0] if children else None

    def get_descendants(
        self,
        node_id: UUID,
        t: NodeT | NodeDataT | None = None,
        recursive: bool = False,
        include_self: bool = False,
    ) -> list["NodeT | NodeDataT"]:
        """Finds all children (or descendants) of the given type"""
        children = [
            self.nodes[child_id]
            for child_id in self.children.get(node_id, [])
            if t is None or isinstance(self.nodes[child_id], t)
        ]
        descendants = children[:]
        if recursive:
            for child in children:
                if child.id not in self.children:
                    continue
                descendants.extend(self.get_descendants(child.id, t, recursive=True))
        if include_self and node_id in self.nodes:
            descendants.append(self.nodes[node_id])
        return descendants

    def get_ancestor(
        self, node_id: UUID, t: NodeT | NodeDataT | None = None
    ) -> Optional["NodeT | NodeDataT"]:
        """Finds the next ancestor of the given type"""
        node = self.nodes.get(node_id)
        if node is None:
            raise ValueError(f"node {node_id} is not in {self}")
        while node:
            if t is None or isinstance(node, t):
                return node
            if node.parent_id is None:
                return None
            node = self.nodes[node.parent_id]
        return None

    def get_ancestors(
        self, node_id: UUID, t: NodeT | NodeDataT | None = None, include_self: bool = False
    ) -> list["NodeT | NodeDataT"]:
        """Finds all ancestors of the given type"""
        ancestors = []
        node = self.nodes.get(node_id)
        if node is None:
            raise ValueError(f"node {node_id} is not in {self}")
        if include_self:
            ancestors.append(node)
        while node:
            if t is None or isinstance(node, t):
                ancestors.append(node)
            if node.parent_id is None:
                break
            node = self.nodes[node.parent_id]
        return ancestors


class NodePacker(abc.ABC, typing.Generic[NodeDataT, NodeT]):
    """Module node packer"""

    def walk(self, node: NodeT, tree: PackContext) -> None:
        """Walks all direct children of the given node"""
        pass

    def pack(self, node: NodeT) -> NodeDataT:
        """Packs the node itself into the wire format"""
        raise NotImplementedError

    def unpack(self, node: NodeDataT, parent: Optional[NodeT], session: Optional[Session]) -> NodeT:
        """Unpacks the node itself from the wire format (plain or instrumented into session)"""
        raise NotImplementedError

    def unwalk(self, node: NodeT, tree: ModuleTree) -> None:
        """Re-assigns the node's children"""
        pass

    def patch(self, node: NodeT, references: dict[UUID, UUID]) -> None:
        """Patches the node's non-parent references (in-tree, in-place)"""
        pass


class PackContext(abc.ABC):
    """Tree visitor for packing"""

    def __init__(self):
        self.visited: dict[UUID, ModuleNode] = {}

    def visit(self, node: ModuleNode):
        self.visited[node.id] = node

    def visit_all(self, nodes: typing.Iterable[ModuleNode]):
        for node in nodes:
            self.visit(node)


# registered packers
_node_packers_by_data: dict[typing.Type[NodeDataT], NodePacker] = {}
_node_packers_by_node: dict[typing.Type[NodeT], NodePacker] = {}
MOT_BY_DATA_CLASS: dict[typing.Type[NodeDataT], MOT] = {}
BASE_DATA_CLASS_BY_MOT: dict[MOT, typing.Type[NodeDataT]] = {}

_DATA_CLASS_BY_NAME: dict[str, typing.Type[NodeDataT]] = {}


def node_packer(t: MOT, data_t: typing.Type[NodeDataT], node_t: typing.Type[NodeT] | None):
    """Decorator to register a node packer for a given type"""

    # unrelated, add data class to data class by name for :WireFormat serialization hack
    _DATA_CLASS_BY_NAME[data_t.__name__] = data_t

    def decorator(cls: "NodePacker"):
        if node_t in _node_packers_by_node:
            raise ValueError(
                f"packer for {node_t} already registered: {_node_packers_by_node[node_t]}"
            )
        if data_t in _node_packers_by_data:
            raise ValueError(
                f"packer for {data_t} already registered: {_node_packers_by_data[data_t]}"
            )
        packer = cls()
        _node_packers_by_node[node_t] = packer
        _node_packers_by_data[data_t] = packer
        MOT_BY_DATA_CLASS[data_t] = t
        if t not in BASE_DATA_CLASS_BY_MOT:
            BASE_DATA_CLASS_BY_MOT[t] = data_t
        elif not issubclass(data_t, BASE_DATA_CLASS_BY_MOT[t]):  # noqa
            raise ValueError(
                f"cannot register {data_t} as {t}, it is not a subclass of {BASE_DATA_CLASS_BY_MOT[t]}"
            )
        return cls

    return decorator


def pack_module(module: lang.Module) -> "ModuleTreeData":
    module_data, nodes = pack_node(module)
    module_tree = ModuleTreeData(**module_data.__dict__, module=module_data, nodes=nodes)
    return module_tree


def unpack_module(module: ModuleTreeData, session: Optional[Session]) -> lang.Module:
    module = unpack_node(module.nodes, parent=None, session=session)
    return module


def walk_node(node: NodeT) -> typing.Iterator[NodeT]:
    """Walks the node and all its descendants"""
    seen: set[UUID] = set()
    to_visit: list[NodeT] = [node]
    while to_visit:
        visited = PackContext()
        for node in to_visit:
            seen.add(node.id)
            yield node
            packer = _node_packers_by_node[type(node)]
            packer.walk(node, visited)
        to_visit = [node for node in visited.visited.values() if node.id not in seen]


def pack_node(root: NodeT) -> tuple[NodeDataT, list[NodeDataT]]:
    """Pack a node and all its descendants"""
    packed: dict[UUID, NodeDataT] = OrderedDict()
    ctx = PackContext()

    # walk and pack until nothing is left to pack
    to_pack: list[NodeT] = [root]
    while to_pack:
        for node in to_pack:
            packer = _node_packers_by_node[type(node)]
            packer.walk(node, ctx)
            packed[node.id] = packer.pack(node)
        to_pack = [node for node in ctx.visited.values() if node.id not in packed]

    return packed[root.id], list(packed.values())


def unpack_node(
    nodes: list[NodeDataT], parent: Optional[NodeT], session: Optional[Session]
) -> NodeT:
    """Unpack a node and all its descendants"""
    data_tree = ModuleTree(nodes)
    unpacked_tree = ModuleTree()

    # unpack all nodes top down (breadth first)
    for node in data_tree.walk_bfs():
        packer = _node_packers_by_data[type(node)]
        if node.parent_id is None:
            node_parent = parent
        elif node.parent_id not in unpacked_tree.nodes:
            if parent is not None and node.parent_id == parent.id:
                node_parent = parent
            else:
                raise ValueError(
                    f"node {node} parent {node.parent_id} not found in unpacked {unpacked_tree}"
                )
        else:
            node_parent = unpacked_tree.nodes[node.parent_id]
        unpacked_tree.add(packer.unpack(node, node_parent, session))

    # 'unwalk' all nodes to re-assign descendants
    for node in unpacked_tree.nodes.values():
        packer = _node_packers_by_node[type(node)]
        packer.unwalk(node, unpacked_tree)

    return unpacked_tree.root


def pack_node_flat(node: NodeT) -> NodeDataT:
    """Pack a language node into a flat module node"""
    packer = _node_packers_by_node[type(node)]
    return packer.pack(node)


def unpack_node_flat(node: NodeDataT, parent: Optional[NodeT], session: Optional[Session]) -> NodeT:
    """Unpack a flat module node into a language node"""
    packer = _node_packers_by_data[type(node)]
    return packer.unpack(node, parent, session)


def patch_node_flat(node: NodeDataT, references: dict[UUID, UUID]) -> NodeDataT:
    """Patch a flat node with out-of-tree-ancestry references"""
    packer = _node_packers_by_data[type(node)]
    packer.patch(node, references)
    return node


@dataclass
class NodeData:
    id: UUID
    parent_id: Optional[UUID]

    @property
    def mot(self) -> ModuleObjectType:
        return MOT_BY_DATA_CLASS[type(self)]

    def equals_ignoring_crud(self, other: "NodeData") -> bool:
        for field in dataclasses.fields(self):
            if field.name in CRUD_PROPERTIES:
                continue
            if getattr(self, field.name) != getattr(other, field.name):
                return False
        return True


@dataclass
class HasCrud:
    created_at: datetime
    updated_at: datetime
    last_edited_at: datetime
    last_changed_at: Optional[datetime]
    revision: int


@dataclass
class HasOrder:
    order_key: str


@dataclass
class ModuleData(NodeData, HasCrud):
    name: str
    committed: bool
    parent_id: Optional[UUID]

    def strip(self) -> ModuleData:
        return replace(self, nodes=None)

    def __str__(self):
        return f"{self.name}@{self.id}"

    def __repr__(self):
        return f"<Module {str(self)}>"


@dataclass
class ModuleTreeData(ModuleData):
    nodes: list[NodeData]
    module: ModuleData

    def __str__(self):
        return f"{self.name}@{self.id}"

    def __repr__(self):
        return f"<Module {str(self)}>"

    def encode_some_attrs(self) -> dict[str, Any]:
        # hack to wire nodes with the type of their base class until :WireFormat
        serialized_nodes = [{**to_dict(node), "cls": type(node).__name__} for node in self.nodes]
        return {"nodes": serialized_nodes}

    @classmethod
    def decode_some_attrs(cls, data: dict[str, Any]) -> dict[str, Any]:
        # hack to serialize nodes with the type of their base class until :WireFormat
        # restore cls from namespace?
        nodes = [from_dict(_DATA_CLASS_BY_NAME[node.pop("cls")], node) for node in data["nodes"]]
        return {"nodes": nodes}


@node_packer(MOT.MODULE, ModuleData, lang.Module)
class ModulePacker(NodePacker[ModuleData, lang.Module]):
    PARENTS: ClassVar[ParentsT] = set()

    def walk(self, module: lang.Module, tree: PackContext):
        for file in module.files:
            tree.visit(file)

    def pack(self, module: lang.Module) -> ModuleData:
        return ModuleData(
            id=module.id,
            name=module.name,
            committed=module.committed,
            parent_id=None,
            revision=module.revision,
            created_at=module.created_at,
            updated_at=module.updated_at,
            last_edited_at=module.last_edited_at,
            last_changed_at=module.last_changed_at,
        )

    def unpack(
        self, module: ModuleData, parent: Optional[lang.Module], session: Optional[Session]
    ) -> lang.Module:
        return lang.Module(
            id=module.id,
            name=module.name,
            committed=module.committed,
            revision=module.revision,
            created_at=module.created_at,
            updated_at=module.updated_at,
            last_edited_at=module.last_edited_at,
            last_changed_at=module.last_changed_at,
            files=[],
        )

    def unwalk(self, module: lang.Module, tree: ModuleTree):
        module.files = tree.get_descendants(module.id, lang.File, recursive=True)


@dataclass
class FileData(NodeData, HasCrud):
    name: str

    def __str__(self):
        return f"{self.name}"

    def __repr__(self):
        return f"<File {str(self)}>"


@node_packer(MOT.FILE, FileData, lang.File)
class FilePacker(NodePacker[FileData, lang.File]):
    PARENTS: ClassVar[ParentsT] = {MOT.MODULE}

    def walk(self, file: lang.File, tree: PackContext):
        for statement in file.statements:
            tree.visit(statement)
        for child in file.children:
            tree.visit(child)

    def pack(self, file: lang.File) -> "FileData":
        return FileData(
            id=file.id,
            parent_id=file.module.id,
            name=file.name,
            revision=file.revision,
            created_at=file.created_at,
            updated_at=file.updated_at,
            last_edited_at=file.last_edited_at,
            last_changed_at=file.last_changed_at,
        )

    def unpack(self, file: FileData, parent: lang.Module, session: Optional[Session]) -> lang.File:
        return lang.File(
            id=file.id,
            module=parent,
            name=file.name,
            revision=file.revision,
            created_at=file.created_at,
            updated_at=file.updated_at,
            last_edited_at=file.last_edited_at,
            last_changed_at=file.last_changed_at,
            statements=[],
            children=[],
            _session=session,
        )

    def unwalk(self, file: lang.File, tree: ModuleTree):
        file.statements = tree.get_descendants(file.id, lang.Statement, recursive=True)
        file.children = tree.get_descendants(file.id, lang.File)


@dataclass
class StatementData(NodeData, HasOrder, HasCrud):
    type: StatementType
    name: Optional[str]

    def __str__(self):
        return f"{self.type.name} {self.name}"

    def __repr__(self):
        return f"<{self.__class__.__name__} {str(self)}>"


@node_packer(MOT.STATEMENT, StatementData, lang.Statement)
class StatementPacker(NodePacker[StatementData, lang.Statement]):
    PARENTS: ClassVar[ParentsT] = {MOT.STATEMENT, MOT.FILE}

    def walk(self, statement: lang.Statement, tree: PackContext):
        for child in statement.children:
            tree.visit(child)

    def pack(self, statement: lang.Statement) -> "StatementData":
        return StatementData(
            id=statement.id,
            parent_id=statement.parent_id,
            order_key=statement.order_key,
            type=statement.type,
            name=statement.name,
            revision=statement.revision,
            created_at=statement.created_at,
            updated_at=statement.updated_at,
            last_edited_at=statement.last_edited_at,
            last_changed_at=statement.last_changed_at,
        )

    def unpack(
        self,
        statement: StatementData,
        parent: lang.File | lang.Statement,
        session: Optional[Session],
    ) -> lang.Statement:
        cls = lang.Blank if statement.type == StatementType.BLANK else lang.Statement
        return cls(
            id=statement.id,
            parent=parent,
            file=parent if isinstance(parent, lang.File) else parent.file,
            children=[],
            order_key=statement.order_key,
            type=statement.type,
            name=statement.name,
            revision=statement.revision,
            created_at=statement.created_at,
            updated_at=statement.updated_at,
            last_edited_at=statement.last_edited_at,
            last_changed_at=statement.last_changed_at,
            _session=session,
        )

    def unwalk(self, statement: lang.Statement, tree: ModuleTree):
        statement.children = tree.get_descendants(statement.id, lang.Statement)


class BlankData(StatementData):
    pass  # it's blank


@node_packer(MOT.STATEMENT, BlankData, lang.Blank)
class BlankPacker(StatementPacker, NodePacker[BlankData, lang.Blank]):
    PARENTS: ClassVar[ParentsT] = {MOT.FILE, MOT.STATEMENT}

    def pack(self, symbol: lang.Blank) -> "BlankData":
        statement_data = super().pack(symbol)
        return BlankData(**statement_data.__dict__)

    def unpack(
        self, statement: BlankData, parent: lang.Statement | lang.File, session: Optional[Session]
    ) -> lang.Blank:
        statement = super().unpack(statement, parent, session)
        return lang.Blank(**statement.__dict__)


@dataclass
class TextData(StatementData):
    text: str


@node_packer(MOT.STATEMENT, TextData, lang.Text)
class TextPacker(StatementPacker, NodePacker[TextData, lang.Text]):
    PARENTS: ClassVar[ParentsT] = {MOT.FILE, MOT.STATEMENT}

    def pack(self, symbol: lang.Text) -> "TextData":
        statement_data = super().pack(symbol)
        return TextData(**statement_data.__dict__, text=symbol.text)

    def unpack(
        self, symbol: TextData, parent: lang.Statement | lang.File, session: Optional[Session]
    ) -> lang.Text:
        statement = super().unpack(symbol, parent, session)
        return lang.Text(**statement.__dict__, text=symbol.text)


@dataclass
class ReferenceData(StatementData):
    reference_id: Optional[UUID]


@node_packer(MOT.STATEMENT, ReferenceData, lang.Reference)
class ReferencePacker(StatementPacker, NodePacker[ReferenceData, lang.Reference]):
    PARENTS: ClassVar[ParentsT] = {MOT.FILE, MOT.STATEMENT}

    def walk(self, statement: lang.Reference, tree: PackContext):
        tree.visit_all(statement.tags)

    def pack(self, symbol: lang.Reference) -> "ReferenceData":
        statement_data = super().pack(symbol)
        return ReferenceData(**statement_data.__dict__, reference_id=symbol.reference_id)

    def unpack(
        self,
        symbol: ReferenceData,
        parent: lang.Statement | lang.File,
        session: Optional[Session],
    ) -> lang.Reference:
        statement = super().unpack(symbol, parent, session)
        return lang.Reference(**statement.__dict__, reference=symbol.reference_id)

    def unwalk(self, statement: lang.Reference, tree: ModuleTree):
        statement.tags = tree.get_descendants(statement.id, lang.Tag)


@dataclass
class BlockData(StatementData):
    description: Optional[str]


@node_packer(MOT.STATEMENT, BlockData, lang.Block)
class BlockPacker(StatementPacker, NodePacker[BlockData, lang.Block]):
    PARENTS: ClassVar[ParentsT] = {MOT.FILE, MOT.STATEMENT}

    def walk(self, statement: lang.Block, tree: PackContext):
        tree.visit_all(statement.tags)

    def pack(self, symbol: lang.Block) -> "BlockData":
        statement_data = super().pack(symbol)
        return BlockData(**statement_data.__dict__, description=symbol.description)

    def unpack(
        self, symbol: BlockData, parent: lang.Statement | lang.File, session: Optional[Session]
    ) -> lang.Block:
        statement = super().unpack(symbol, parent, session)
        return lang.Block(**statement.__dict__, description=symbol.description)

    def unwalk(self, statement: lang.Block, tree: ModuleTree):
        statement.tags = tree.get_descendants(statement.id, lang.Tag)


@dataclass
class TypeData(StatementData):
    tag: Optional[TypeTag]
    key: Optional[str]
    flags: Optional[TypeFlag]
    description: Optional[str]


@node_packer(MOT.STATEMENT, TypeData, lang.Type)
class TypePacker(StatementPacker, NodePacker[TypeData, lang.Type]):
    PARENTS: ClassVar[ParentsT] = {MOT.FILE, MOT.STATEMENT}

    def walk(self, symbol: lang.Type, tree: PackContext):
        super().walk(symbol, tree)
        tree.visit_all(symbol.fields)
        tree.visit_all(symbol.tags)

    def pack(self, symbol: lang.Type) -> "TypeData":
        statement_data = super().pack(symbol)
        return TypeData(
            **statement_data.__dict__,
            key=symbol.key,
            description=symbol.description,
            tag=symbol.tag,
            flags=symbol.flags,
        )

    def unpack(
        self, symbol: TypeData, parent: lang.File | lang.Statement, session: Optional[Session]
    ) -> lang.Type:
        statement = super().unpack(symbol, parent, session)
        return lang.Type(
            **statement.__dict__,
            key=symbol.key,
            description=symbol.description,
            tag=symbol.tag,
            flags=symbol.flags,
            fields=[],
            tags=[],
        )

    def unwalk(self, symbol: lang.Type, tree: ModuleTree):
        super().unwalk(symbol, tree)
        symbol.fields = tree.get_descendants(symbol.id, lang.Field)
        symbol.tags = tree.get_descendants(symbol.id, lang.Tagging)


@dataclass
class TagData(StatementData):
    description: Optional[str]
    key: str


@node_packer(MOT.STATEMENT, TagData, lang.Tag)
class TagPacker(StatementPacker, NodePacker[TagData, lang.Tag]):
    PARENTS: ClassVar[ParentsT] = {MOT.FILE, MOT.STATEMENT}

    def walk(self, symbol: lang.Tag, tree: PackContext):
        super().walk(symbol, tree)
        tree.visit_all(symbol.fields)
        tree.visit_all(symbol.tags)

    def pack(self, symbol: lang.Tag) -> "TagData":
        statement_data = super().pack(symbol)
        return TagData(
            **statement_data.__dict__,
            description=symbol.description,
            key=symbol.key,
        )

    def unpack(
        self, symbol: TagData, parent: lang.File | lang.Statement, session: Optional[Session]
    ) -> lang.Tag:
        statement = super().unpack(symbol, parent, session)
        return lang.Tag(
            **statement.__dict__,
            description=symbol.description,
            key=symbol.key,
            fields=[],
            tags=[],
        )

    def unwalk(self, symbol: lang.Tag, tree: ModuleTree):
        super().unwalk(symbol, tree)
        symbol.fields = tree.get_descendants(symbol.id, lang.Field)
        symbol.tags = tree.get_descendants(symbol.id, lang.Tagging)


@dataclass
class TaskData(StatementData):
    description: Optional[str]


@node_packer(MOT.STATEMENT, TaskData, lang.Task)
class TaskPacker(StatementPacker, NodePacker[TaskData, lang.Task]):
    PARENTS: ClassVar[ParentsT] = {MOT.FILE, MOT.STATEMENT}

    def walk(self, symbol: lang.Task, tree: PackContext):
        super().walk(symbol, tree)
        tree.visit_all(symbol.fields)
        tree.visit_all(symbol.tags)

    def pack(self, symbol: lang.Task) -> "TaskData":
        statement_data = super().pack(symbol)
        return TaskData(
            **statement_data.__dict__,
            description=symbol.description,
        )

    def unpack(
        self, symbol: TaskData, parent: lang.File | lang.Statement, session: Optional[Session]
    ) -> lang.Task:
        statement = super().unpack(symbol, parent, session)
        return lang.Task(
            **statement.__dict__,
            description=symbol.description,
            fields=[],
            tags=[],
        )

    def unwalk(self, symbol: lang.Task, tree: ModuleTree):
        super().unwalk(symbol, tree)
        symbol.fields = tree.get_descendants(symbol.id, lang.Field)
        symbol.tags = tree.get_descendants(symbol.id, lang.Tagging)


@dataclass
class ExpectationData(StatementData):
    description: Optional[str]
    reference_id: Optional[UUID]


@node_packer(MOT.STATEMENT, ExpectationData, lang.Expectation)
class ExpectationPacker(StatementPacker, NodePacker[ExpectationData, lang.Expectation]):
    PARENTS: ClassVar[ParentsT] = {MOT.FILE, MOT.STATEMENT}

    def walk(self, symbol: lang.Expectation, tree: PackContext):
        super().walk(symbol, tree)
        tree.visit_all(symbol.tags)

    def pack(self, symbol: lang.Expectation) -> "ExpectationData":
        statement_data = super().pack(symbol)
        return ExpectationData(
            **statement_data.__dict__,
            description=symbol.description,
            reference_id=symbol.reference.id
            if isinstance(symbol.reference, lang.Statement)
            else symbol.reference,
        )

    def unpack(
        self,
        symbol: ExpectationData,
        parent: lang.File | lang.Statement,
        session: Optional[Session],
    ) -> lang.Expectation:
        statement = super().unpack(symbol, parent, session)
        return lang.Expectation(
            **statement.__dict__,
            description=symbol.description,
            reference=symbol.reference_id,
        )

    def unwalk(self, statement: lang.Expectation, tree: ModuleTree):
        super().unwalk(statement, tree)
        statement.tags = tree.get_descendants(statement.id, lang.Tag)

    def patch(self, symbol: ExpectationData, references: dict[UUID, UUID]):
        super().patch(symbol, references)
        symbol.reference_id = references.get(symbol.reference_id, symbol.reference_id)


@dataclass
class CodeData(StatementData):
    language: Optional[str]
    description: Optional[str]
    code: Optional[str]


@node_packer(MOT.STATEMENT, CodeData, lang.Code)
class CodePacker(StatementPacker, NodePacker[CodeData, lang.Code]):
    PARENTS: ClassVar[ParentsT] = {MOT.FILE, MOT.STATEMENT}

    def walk(self, symbol: lang.Code, tree: PackContext):
        super().walk(symbol, tree)
        tree.visit_all(symbol.fields)
        tree.visit_all(symbol.tags)

    def pack(self, symbol: lang.Code) -> "CodeData":
        statement_data = super().pack(symbol)
        return CodeData(
            **statement_data.__dict__,
            language=symbol.language,
            description=symbol.description,
            code=symbol.code,
        )

    def unpack(
        self, symbol: CodeData, parent: lang.File | lang.Statement, session: Optional[Session]
    ) -> lang.Code:
        statement = super().unpack(symbol, parent, session)
        return lang.Code(
            **statement.__dict__,
            fields=[],
            tags=[],
            language=symbol.language,
            description=symbol.description,
            code=symbol.code,
        )

    def unwalk(self, symbol: lang.Code, tree: ModuleTree):
        super().unwalk(symbol, tree)
        symbol.fields = tree.get_descendants(symbol.id, lang.Field)
        symbol.tags = tree.get_descendants(symbol.id, lang.Tagging)


@dataclass
class ModelData(StatementData):
    external_name: Optional[str]
    description: Optional[str]


@node_packer(MOT.STATEMENT, ModelData, lang.Model)
class ModelPacker(StatementPacker, NodePacker[ModelData, lang.Model]):
    PARENTS: ClassVar[ParentsT] = {MOT.FILE, MOT.STATEMENT}

    def walk(self, symbol: lang.Value, tree: PackContext):
        super().walk(symbol, tree)
        tree.visit_all(symbol.fields)
        tree.visit_all(symbol.tags)

    def pack(self, symbol: lang.Model) -> "ModelData":
        statement_data = super().pack(symbol)
        return ModelData(
            **statement_data.__dict__,
            external_name=symbol.external_name,
            description=symbol.description,
        )

    def unpack(
        self, symbol: ModelData, parent: lang.File | lang.Statement, session: Optional[Session]
    ) -> lang.Model:
        statement = super().unpack(symbol, parent, session)
        return lang.Model(
            **statement.__dict__,
            external_name=symbol.external_name,
            description=symbol.description,
            fields=[],
            tags=[],
        )

    def unwalk(self, symbol: lang.Value, tree: ModuleTree):
        super().unwalk(symbol, tree)
        symbol.fields = tree.get_descendants(symbol.id, lang.Field)
        symbol.tags = tree.get_descendants(symbol.id, lang.Tagging)


@dataclass
class ValueData(StatementData):
    description: Optional[str]
    value: Optional[typing.Any]


@node_packer(MOT.STATEMENT, ValueData, lang.Value)
class ValuePacker(StatementPacker, NodePacker[ValueData, lang.Value]):
    PARENTS: ClassVar[ParentsT] = {MOT.FILE, MOT.STATEMENT}

    def walk(self, symbol: lang.Value, tree: PackContext):
        super().walk(symbol, tree)
        tree.visit_all(symbol.fields)
        tree.visit_all(symbol.tags)

    def pack(self, symbol: lang.Value) -> "ValueData":
        statement_data = super().pack(symbol)
        return ValueData(
            **statement_data.__dict__,
            description=symbol.description,
            value=symbol._raw_value(),
        )

    def unpack(
        self, symbol: ValueData, parent: lang.File | lang.Statement, session: Optional[Session]
    ) -> lang.Value:
        statement = super().unpack(symbol, parent, session)
        return lang.Value(
            **statement.__dict__,
            description=symbol.description,
            value=symbol.value,
            _instantiated=False,
        )

    def unwalk(self, symbol: lang.Value, tree: ModuleTree):
        super().unwalk(symbol, tree)
        symbol.fields = tree.get_descendants(symbol.id, lang.Field)
        symbol.tags = tree.get_descendants(symbol.id, lang.Tagging)


@dataclass
class DatasetData(StatementData):
    description: Optional[str]
    versioned: bool
    backend: DatasetBackend
    backend_id: str


@node_packer(MOT.STATEMENT, DatasetData, lang.Dataset)
class DatasetPacker(StatementPacker, NodePacker[DatasetData, lang.Dataset]):
    PARENTS: ClassVar[ParentsT] = {MOT.FILE, MOT.STATEMENT}

    def walk(self, symbol: lang.Dataset, tree: PackContext):
        super().walk(symbol, tree)
        tree.visit_all(symbol.fields)
        tree.visit_all(symbol.tags)

    def pack(self, symbol: lang.Dataset) -> "DatasetData":
        statement_data = super().pack(symbol)
        return DatasetData(
            **statement_data.__dict__,
            description=symbol.description,
            versioned=symbol.versioned,
            backend=symbol.backend,
            backend_id=symbol.backend_id,
        )

    def unpack(
        self, symbol: DatasetData, parent: lang.Statement, session: Optional[Session]
    ) -> lang.Dataset:
        statement = super().unpack(symbol, parent, session)
        return lang.Dataset(
            **statement.__dict__,
            description=symbol.description,
            versioned=symbol.versioned,
            fields=[],
            tags=[],
            backend=symbol.backend,
            backend_id=symbol.backend_id,
        )

    def unwalk(self, symbol: lang.Dataset, tree: ModuleTree):
        super().unwalk(symbol, tree)
        symbol.fields = tree.get_descendants(symbol.id, lang.Field)
        symbol.tags = tree.get_descendants(symbol.id, lang.Tagging)


STATEMENT_DATA_BY_TYPE = {
    StatementType.TYPE: TypeData,
    StatementType.TASK: TaskData,
    StatementType.EXPECTATION: ExpectationData,
    StatementType.CODE: CodeData,
    StatementType.MODEL: ModelData,
    StatementType.DATASET: DatasetData,
    StatementType.VALUE: ValueData,
}
STATEMENT_TYPE_BY_DATA_CLASS = {v: k for k, v in STATEMENT_DATA_BY_TYPE.items()}


@dataclass
class FieldData(NodeData, HasOrder, HasCrud):
    name: Optional[str]
    key: str
    tag: TypeTag
    hint: Optional[TypeHint]
    description: Optional[str]
    flags: TypeFlag
    reference_id: Optional[UUID] = None
    metadata: Optional[typing.Any] = None

    def __str__(self):
        name_str = f"{self.name} " if self.name else ""
        return f"{self.parent_id}:{self.order_key} {name_str}{self.tag.value}"

    def __repr__(self):
        return f"<Field {str(self)}>"


@node_packer(MOT.FIELD, FieldData, lang.Field)
class FieldPacker(NodePacker[FieldData, lang.Field]):
    PARENTS: ClassVar[ParentsT] = {MOT.FILE, MOT.STATEMENT}

    def pack(self, field: lang.Field) -> "FieldData":
        reference = field.reference.id if isinstance(field.reference, lang.Statement) else None
        return FieldData(
            id=field.id,
            parent_id=field.parent_id,
            order_key=field.order_key,
            name=field.name,
            key=field.key,
            tag=field.tag,
            hint=field.hint,
            flags=field.flags,
            description=field.description,
            reference_id=reference,
            metadata=field.metadata,
            revision=field.revision,
            created_at=field.created_at,
            updated_at=field.updated_at,
            last_edited_at=field.last_edited_at,
            last_changed_at=field.last_changed_at,
        )

    def unpack(
        self, field: FieldData, parent: lang.Statement, session: Optional[Session]
    ) -> lang.Field:
        return lang.Field(
            parent=parent,
            id=field.id,
            name=field.name,
            key=field.key,
            tag=field.tag,
            hint=field.hint,
            flags=field.flags,
            description=field.description,
            reference=field.reference_id,
            metadata=field.metadata,
            revision=field.revision,
            created_at=field.created_at,
            updated_at=field.updated_at,
            last_edited_at=field.last_edited_at,
            last_changed_at=field.last_changed_at,
            _session=session,
        )

    def patch(self, field: FieldData, references: dict[UUID, UUID]) -> None:
        field.reference_id = references.get(field.reference_id, field.reference_id)


@dataclass
class TaggingData(NodeData, HasCrud):
    reference_id: Optional[UUID]
    key: str
    metadata: Optional[typing.Any] = None

    def __str__(self):
        return self.key

    def __repr__(self):
        return f"<Tagging {self}>"


@node_packer(MOT.TAGGING, TaggingData, lang.Tagging)
class TaggingPacker(NodePacker[TaggingData, lang.Tagging]):
    PARENTS: ClassVar[ParentsT] = {MOT.FILE, MOT.STATEMENT}

    def pack(self, tagging: lang.Tagging) -> "TaggingData":
        reference = tagging.reference.id if isinstance(tagging.reference, lang.Statement) else None
        return TaggingData(
            id=tagging.id,
            parent_id=tagging.parent_id,
            reference_id=reference,
            key=tagging.key,
            metadata=tagging.metadata,
            revision=tagging.revision,
            created_at=tagging.created_at,
            updated_at=tagging.updated_at,
            last_edited_at=tagging.last_edited_at,
            last_changed_at=tagging.last_changed_at,
        )

    def unpack(
        self, tagging: TaggingData, parent: lang.Statement, session: Optional[Session]
    ) -> lang.Tagging:
        return lang.Tagging(
            parent=parent,
            id=tagging.id,
            key=tagging.key,
            reference=tagging.reference_id,
            metadata=tagging.metadata,
            revision=tagging.revision,
            created_at=tagging.created_at,
            updated_at=tagging.updated_at,
            last_edited_at=tagging.last_edited_at,
            last_changed_at=tagging.last_changed_at,
            _session=session,
        )

    def patch(self, tagging: TaggingData, references: dict[UUID, UUID]) -> None:
        tagging.reference_id = references.get(tagging.reference_id, tagging.reference_id)


@dataclass
class DatasetViewData(NodeData, HasOrder, HasCrud):
    PARENTS: ClassVar[ParentsT] = {MOT.FILE, MOT.STATEMENT}

    id: UUID
    name: str
    query: Optional[Query] = None
    sort: Optional[list[Sort]] = None
    length: Optional[int] = None
    reference_id: Optional[UUID] = None


@node_packer(MOT.DATASET_VIEW, DatasetViewData, lang.DatasetView)
class DatasetViewPacker(NodePacker[DatasetViewData, lang.DatasetView]):
    PARENTS: ClassVar[ParentsT] = {MOT.STATEMENT}

    def pack(self, view: lang.DatasetView) -> "DatasetViewData":
        return DatasetViewData(
            id=view.id,
            order_key=view.order_key,
            name=view.name,
            query=view.query,
            sort=view.sort,
            reference_id=view.reference.id if view.reference else None,
            revision=view.revision,
            created_at=view.created_at,
            updated_at=view.updated_at,
            last_edited_at=view.last_edited_at,
            last_changed_at=view.last_changed_at,
        )

    def unpack(
        self, view: DatasetViewData, parent: lang.Statement, session: Optional[Session]
    ) -> lang.DatasetView:
        return lang.DatasetView(
            id=view.id,
            name=view.name,
            source=parent,
            query=view.query,
            sort=view.sort,
            reference=view.reference_id,
            revision=view.revision,
            created_at=view.created_at,
            updated_at=view.updated_at,
            last_edited_at=view.last_edited_at,
            last_changed_at=view.last_changed_at,
            _session=session,
        )

    def patch(self, view: DatasetViewData, references: dict[UUID, UUID]) -> None:
        view.reference_id = references.get(view.reference_id, view.reference_id)


@dataclass
class RecordData(NodeData, HasOrder, HasCrud):
    PARENTS: ClassVar[ParentsT] = {MOT.STATEMENT}

    value: Optional[typing.Any] = None

    def __str__(self):
        return f"{self.parent_id}:{self.order_key} {describe_type(self.value)}"

    def __repr__(self):
        return f"<{self.__class__.__name__} {str(self)}>"


@node_packer(MOT.RECORD, RecordData, lang.Record)
class RecordPacker(NodePacker[RecordData, lang.Record]):
    PARENTS: ClassVar[ParentsT] = {MOT.STATEMENT}

    def pack(self, record: lang.Record) -> "RecordData":
        return RecordData(
            id=record.id,
            parent_id=record.parent_id,
            order_key=record.order_key,
            value=record._raw_value(),
            revision=record.revision,
            created_at=record.created_at,
            updated_at=record.updated_at,
            last_edited_at=record.last_edited_at,
            last_changed_at=record.last_changed_at,
        )

    def unpack(
        self, record: "RecordData", parent: lang.Statement, session: Optional[Session]
    ) -> lang.Record:
        return lang.Record(
            id=record.id,
            parent=parent,
            value=record.value,
            order_key=record.order_key,
            revision=record.revision,
            created_at=record.created_at,
            updated_at=record.updated_at,
            last_edited_at=record.last_edited_at,
            last_changed_at=record.last_changed_at,
            _instantiated=False,
        )


@dataclass
class ResolvedFieldData(NodeData):
    field_id: UUID


@node_packer(MOT.RESOLVED_FIELD, ResolvedFieldData, lang.ResolvedField)
class ResolvedFieldPacker(NodePacker[ResolvedFieldData, lang.ResolvedField]):
    PARENTS: ClassVar[ParentsT] = {MOT.STATEMENT}

    def pack(self, resolved_field: lang.ResolvedField) -> "ResolvedFieldData":
        return ResolvedFieldData(
            id=resolved_field.id,
            parent_id=resolved_field.parent_id,
            field_id=resolved_field.field.id,
        )


@dataclass
class IssueData(NodeData):
    scope: InterpScope
    kind: IssueKind
    type: IssueType
    message: Optional[str]


@node_packer(MOT.ISSUE, IssueData, lang.Issue)
class IssuePacker(NodePacker[IssueData, lang.Issue]):
    PARENTS: ClassVar[ParentsT] = {MOT.STATEMENT}

    def pack(self, issue: lang.Issue) -> "IssueData":
        return IssueData(
            id=issue.id,
            parent_id=issue.statement_id or issue.file_id,
            scope=issue.scope,
            kind=issue.kind,
            type=issue.type,
            message=issue.message,
        )

    def unpack(
        self, issue: IssueData, parent: lang.Statement, session: Optional[Session]
    ) -> lang.Issue:
        raise NotImplementedError


# other objects


class DataPacker(abc.ABC, typing.Generic[DataT, ObjectT]):
    """Generic data packer for non-node data types"""

    def pack(self, object: ObjectT) -> DataT:
        raise NotImplementedError

    def unpack(self, data: DataT) -> ObjectT:
        raise NotImplementedError


_data_packers_by_data: dict[typing.Type[DataT], "DataPacker"] = {}
_data_packers_by_node: dict[typing.Type, "DataPacker"] = {}


def data_packer(data_t: typing.Type[DataT], node_t: typing.Optional[typing.Type] | None):
    """Decorator to register a data packer for a given type"""

    def decorator(cls: "DataPacker"):
        if data_t in _data_packers_by_data:
            raise ValueError(
                f"packer for {data_t} already registered: {_data_packers_by_data[data_t]}"
            )
        if node_t in _data_packers_by_node:
            raise ValueError(
                f"packer for {node_t} already registered: {_data_packers_by_node[node_t]}"
            )
        packer = cls()
        _data_packers_by_data[data_t] = packer
        if node_t:
            _data_packers_by_node[node_t] = packer
        return cls

    return decorator


def pack_data(data: ObjectT) -> DataT:
    """Pack a language data object into a flat module node"""
    packer = _data_packers_by_node[type(data)]
    return packer.pack(data)


def unpack_data(data: DataT) -> ObjectT:
    """Unpack a flat module node into a language data object"""
    packer = _data_packers_by_data[type(data)]
    return packer.unpack(data)


@dataclass
class RemoteObjectData:
    id: UUID
    sha512: str
    content_length: int
    content_type: str
    name: Optional[str]
    status: RemoteObjectStatus

    def __str__(self):
        return f"{self.id} {self.name} ({self.content_type}, {self.content_length} bytes)"

    def __repr__(self):
        return f"<RemoteObject {self}>"


@data_packer(RemoteObjectData, lang.RemoteObject)
class RemoteObjectPacker(DataPacker[RemoteObjectData, lang.RemoteObject]):
    def pack(self, object: lang.RemoteObject) -> RemoteObjectData:
        return RemoteObjectData(
            id=object.id,
            sha512=object.sha512,
            content_length=object.content_length,
            content_type=object.content_type,
            name=object.name,
            status=object.status,
        )

    def unpack(self, data: RemoteObjectData) -> lang.RemoteObject:
        return lang.RemoteObject(
            id=data.id,
            sha512=data.sha512,
            content_length=data.content_length,
            content_type=data.content_type,
            name=data.name,
            status=data.status,
        )


@dataclass
class SecretData:
    id: UUID
    sha512: str
    value: Optional[typing.Any] = None

    def __str__(self):
        return f"{self.id} ({self.sha512})"

    def __repr__(self):
        return f"<Secret {self}>"


@data_packer(SecretData, lang.Secret)
class SecretPacker(DataPacker[SecretData, lang.Secret]):
    def pack(self, object: lang.Secret) -> SecretData:
        return SecretData(id=object.id, sha512=object.sha512, value=object.value)

    def unpack(self, data: SecretData) -> lang.Secret:
        return lang.Secret(id=data.id, sha512=data.sha512, value=data.value)


@dataclass
class ExecutionFrameData:
    """Wire-able representation of an execution frame."""

    id: UUID
    module_id: UUID
    runnable_id: UUID
    root_id: Optional[UUID]
    parent_id: Optional[UUID]
    entered_at: datetime
    exited_at: Optional[datetime]
    cached_generated_at: Optional[datetime]
    cached_duration: Optional[float]
    inputs: Optional[Any]
    outputs: Optional[Any]
    error: Optional[RunError]
    queue_position: Optional[int]
    # additional context data not in ExecutionFrame
    project_id: UUID
    tracing_level: Optional[int]
    worker_id: UUID
    trigger_type: Optional[ExecutionTriggerType]
    trigger_id: Optional[UUID]

    @staticmethod
    def from_frame(frame: ExecutionFrame, *, session: "Session") -> ExecutionFrameData:
        # this should also follow the packer pattern
        if frame.error:
            if frame.runnable is None:
                raise ValueError(f"error outside code: {frame}")
            stack_summary = traceback.StackSummary.extract(
                traceback.walk_tb(frame.error.__traceback__), capture_locals=True
            )
            if isinstance(frame.runnable, lang.Code):
                stack = ExecutionCodeFrame.from_stack(stack_summary)
                stack = ExecutionCodeFrame.clean(stack, frame.runnable, session=session)
            else:
                stack = []
            error_str = str(frame.error)
            # remove (source=...) from error message
            error_str = re.sub(r"\(source=.+\)", "", error_str)
            error_data = RunError(
                kind=RunErrorKind.RUNTIME,
                type=type(frame.error).__name__,
                statement_id=frame.runnable.id,
                message=f"{type(frame.error).__name__}: {error_str}",
                traceback=stack,
            )
        else:
            error_data = None
        return ExecutionFrameData(
            id=frame.id,
            module_id=frame.module_id,
            runnable_id=frame.runnable.id if frame.runnable else None,
            root_id=frame.root.id if frame.root else None,
            parent_id=frame.parent.id if frame.parent else None,
            entered_at=frame.entered_at,
            exited_at=frame.exited_at,
            cached_generated_at=frame.cached_generated_at,
            cached_duration=frame.cached_duration,
            inputs=frame.inputs,
            outputs=frame.outputs,
            error=error_data,
            queue_position=frame.queue_position,
            project_id=session.ctx.project_id,
            tracing_level=session.ctx.tracing_level,
            worker_id=session.ctx.worker_id,
            trigger_type=session.ctx.trigger_type,
            trigger_id=session.ctx.trigger_id,
        )
