from __future__ import annotations

import abc
import dataclasses
import typing
from collections import OrderedDict, deque
from dataclasses import dataclass, replace
from datetime import datetime
from typing import Any, ClassVar, Optional
from uuid import UUID

import msgpack

import bench.language.run
from bench import language as lang
from bench.language import File, IssueType, Module
from bench.language.const import (
    MNT,
    IssueKind,
    ModuleNodeType,
    RemoteObjectStatus,
    RunStatus,
    ScheduleType,
    StatementType,
    TextHeadingLevel,
    TriggerType,
    TypeFlag,
    TypeHint,
    TypeTag,
    WorkerProfile,
    WorkerRegion,
    WorkerSetStatus,
)
from bench.language.module import ModuleNode, ModuleVisitor
from bench.language.query import Query, Sort
from bench.language.run import RunCodeFrame, RunErrorKind
from bench.language.session import LazyRun, Session
from bench.language.statement import STATEMENT_CLASS_BY_TYPE
from bench.language.text import patch_text_html
from bench.utils.func import describe_type
from bench.utils.serialize import from_dict, to_dict

#
# Stable, concise and flat data nodes for transit and storage.
# TODO @Performance @Robustness: use an optimized and evolvable :WireFormat
# TODO! @Cleanup @Architecture: auto-generate wire format and module data packers (most of it)
#  We can probably do this and implement protobuf or such at the same time.
#  Maybe we can also auto-generate some of the model packers, though that mapping is less 1:1.
#


ParentsT = set[MNT]
NodeDataT = typing.TypeVar("NodeDataT", bound="NodeData")
NodeT = typing.TypeVar("NodeT", bound=ModuleNode)
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

    def path_of(self, node: NodeT | NodeDataT) -> list[NodeT | NodeDataT]:
        """Returns the path from the root to the node"""
        path = []
        while node:
            path.insert(0, node)
            node = self.nodes.get(node.parent_id)
        return path

    def add(self, node: NodeT | NodeDataT):
        """Add a node to the tree (error if node already exists)"""
        if node.id is None:
            raise ValueError(f"node {node} has no id")
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
        t: type[NodeT] | type[NodeDataT] | None = None,
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
        self, node_id: UUID, t: type[NodeT] | type[NodeDataT] | None = None
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
        self,
        node_id: UUID,
        t: type[NodeT] | type[NodeDataT] | None = None,
        include_self: bool = False,
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

    def pack(self, node: NodeT) -> NodeDataT:
        """Packs the node itself into the wire format"""
        raise NotImplementedError(f"pack not implemented for {self.__class__.__name__}")

    def unpack(self, node: NodeDataT, parent: Optional[NodeT], session: Optional[Session]) -> NodeT:
        """Unpacks the node itself from the wire format (plain or instrumented into session)"""
        raise NotImplementedError(f"unpack not implemented for {self.__class__.__name__}")

    def recover(self, node: NodeT, tree: ModuleTree) -> None:
        """Re-assigns the node's children"""
        pass

    def patch(self, node: NodeDataT, target_cks: dict[UUID, UUID]) -> None:
        pass


class PackContext(ModuleVisitor):
    """Tree visitor for packing"""

    pass


# registered packers
_node_packers_by_data: dict[typing.Type[NodeDataT], NodePacker] = {}
_node_packers_by_node: dict[typing.Type[NodeT], NodePacker] = {}
MNT_BY_DATA_CLASS: dict[typing.Type[NodeDataT], MNT] = {}
DATA_CLASS_BY_MNT: dict[MNT, typing.Type[NodeDataT]] = {}

_DATA_CLASS_BY_NAME: dict[str, typing.Type[NodeDataT]] = {}


def node_packer(
    mnt: MNT,
    data_t: typing.Type[NodeDataT],
    node_t: typing.Type[NodeT] | None,
    extra_classes: typing.Collection[typing.Type[NodeT]] = None,
):
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
        if mnt in DATA_CLASS_BY_MNT:
            raise ValueError(f"packer for {mnt} already registered: {DATA_CLASS_BY_MNT[mnt]}")
        packer = cls()
        for nt in [node_t] + list(extra_classes or []):
            _node_packers_by_node[nt] = packer
        _node_packers_by_data[data_t] = packer
        MNT_BY_DATA_CLASS[data_t] = mnt
        DATA_CLASS_BY_MNT[mnt] = data_t
        return cls

    return decorator


def pack_module(module: Module) -> "ModuleTreeData":
    module_data, nodes = pack_node(module)
    module_tree = ModuleTreeData(**module_data.__dict__, module=module_data, nodes=nodes)
    return module_tree


def unpack_module(module: ModuleTreeData, session: Optional[Session]) -> Module:
    module = unpack_node(module.nodes, parent=None, session=session)
    return module


def pack_node(root: NodeT) -> tuple[NodeDataT, list[NodeDataT]]:
    """Pack a node and all its descendants"""
    packed: dict[UUID, NodeDataT] = OrderedDict()
    ctx = PackContext()

    # walk and pack until nothing is left to pack
    to_pack: list[NodeT] = [root]
    while to_pack:
        for node in to_pack:
            node._visit(ctx)
            packer = _node_packers_by_node[type(node)]
            packed_node = packer.pack(node)
            packed[node.id] = packed_node

        to_pack = [node for node in ctx.subtree if node.id not in packed]

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
        packer.recover(node, unpacked_tree)

    return unpacked_tree.root


def pack_node_flat(node: NodeT) -> NodeDataT:
    """Pack a language node into a flat module node"""
    packer = _node_packers_by_node[type(node)]
    return packer.pack(node)


def unpack_node_flat(node: NodeDataT, parent: Optional[NodeT], session: Optional[Session]) -> NodeT:
    """Unpack a flat module node into a language node"""
    packer = _node_packers_by_data[type(node)]
    return packer.unpack(node, parent, session)


def patch_node_flat(node: NodeDataT, target_cks: dict[UUID, UUID]) -> NodeDataT:
    """Patch a flat module node"""
    packer = _node_packers_by_data[type(node)]
    packer.patch(node, target_cks)
    return node


@dataclass
class NodeData:
    id: UUID
    ck: UUID
    parent_id: Optional[UUID]

    @property
    def mnt(self) -> ModuleNodeType:
        return MNT_BY_DATA_CLASS[type(self)]

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


CRUD_PROPERTIES: set[str] = {f.name for f in dataclasses.fields(HasCrud)}


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


@node_packer(MNT.Module, ModuleData, Module)
class ModulePacker(NodePacker[ModuleData, Module]):
    PARENTS: ClassVar[ParentsT] = set()

    def pack(self, module: Module) -> ModuleData:
        return ModuleData(
            id=module.id,
            ck=module.ck,
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
        self,
        module: ModuleData,
        parent: Optional[Module],
        session: Optional[Session],
    ) -> Module:
        return Module(
            id=module.id,
            ck=module.ck,
            name=module.name,
            committed=module.committed,
            revision=module.revision,
            created_at=module.created_at,
            updated_at=module.updated_at,
            last_edited_at=module.last_edited_at,
            last_changed_at=module.last_changed_at,
            files=[],
        )

    def recover(self, module: Module, tree: ModuleTree):
        module.files = tree.get_descendants(module.id, File, recursive=True)


@dataclass
class FileData(NodeData, HasCrud):
    name: str

    def __str__(self):
        return f"{self.name}"

    def __repr__(self):
        return f"<File {str(self)}>"


@node_packer(MNT.File, FileData, File)
class FilePacker(NodePacker[FileData, File]):
    PARENTS: ClassVar[ParentsT] = {MNT.Module}

    def pack(self, file: File) -> "FileData":
        return FileData(
            id=file.id,
            ck=file.ck,
            parent_id=file.module.id,
            name=file.name,
            revision=file.revision,
            created_at=file.created_at,
            updated_at=file.updated_at,
            last_edited_at=file.last_edited_at,
            last_changed_at=file.last_changed_at,
        )

    def unpack(self, file: FileData, parent: Module, session: Optional[Session]) -> File:
        return File(
            id=file.id,
            ck=file.ck,
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

    def recover(self, file: File, tree: ModuleTree):
        file.statements = tree.get_descendants(file.id, lang.Statement, recursive=True)
        file.children = tree.get_descendants(file.id, File)


@dataclass
class StatementData(NodeData, HasOrder, HasCrud):
    type: StatementType
    name: Optional[str]
    heading_level: Optional[TextHeadingLevel]
    text: Optional[str]
    tag: Optional[TypeTag]
    key: Optional[str]
    flags: Optional[TypeFlag]
    code: Optional[str]
    value: Optional[typing.Any]
    versioned: Optional[bool]
    reference_ck: Optional[UUID]

    def __str__(self):
        return f"{self.type.name} {self.name}"

    def __repr__(self):
        return f"<{self.__class__.__name__} {str(self)}>"


@node_packer(
    MNT.Statement, StatementData, lang.Statement, extra_classes=STATEMENT_CLASS_BY_TYPE.values()
)
class StatementPacker(NodePacker[StatementData, lang.Statement]):
    PARENTS: ClassVar[ParentsT] = {MNT.Statement, MNT.File}

    def pack(self, statement: lang.Statement) -> "StatementData":
        value = statement._raw_value() if isinstance(statement, lang.HasValue) else statement.value
        return StatementData(
            id=statement.id,
            ck=statement.ck,
            parent_id=statement.parent_id,
            order_key=statement.order_key,
            type=statement.type,
            name=statement.name,
            heading_level=statement.heading_level,
            text=statement.text,
            tag=statement.tag,
            key=statement.key,
            flags=statement.flags,
            code=statement.code,
            value=value,
            versioned=statement.versioned,
            reference_ck=statement.reference_ck,
            revision=statement.revision,
            created_at=statement.created_at,
            updated_at=statement.updated_at,
            last_edited_at=statement.last_edited_at,
            last_changed_at=statement.last_changed_at,
        )

    def unpack(
        self,
        statement: StatementData,
        parent: File | lang.Statement,
        session: Optional[Session],
    ) -> lang.Statement:
        cls = STATEMENT_CLASS_BY_TYPE[statement.type]
        return cls(
            id=statement.id,
            ck=statement.ck,
            parent=parent,
            file=parent if isinstance(parent, File) else parent.file,
            children=[],
            order_key=statement.order_key,
            type=statement.type,
            name=statement.name,
            heading_level=statement.heading_level,
            text=statement.text,
            tag=statement.tag or cls.tag,
            key=statement.key,
            flags=statement.flags or cls.flags,
            code=statement.code,
            value=statement.value,
            versioned=statement.versioned,
            reference=statement.reference_ck,
            revision=statement.revision,
            created_at=statement.created_at,
            updated_at=statement.updated_at,
            last_edited_at=statement.last_edited_at,
            last_changed_at=statement.last_changed_at,
            _session=session,
        )

    def recover(self, statement: lang.Statement, tree: ModuleTree):
        statement.children = tree.get_descendants(statement.id, lang.Statement)
        if isinstance(statement, lang.HasTags):
            statement.tags = tree.get_descendants(statement.id, lang.Tagging)
        if isinstance(statement, lang.HasTriggers):
            statement.triggers = tree.get_descendants(statement.id, lang.Trigger)
        if isinstance(statement, lang.HasFields):
            statement.fields = tree.get_descendants(statement.id, lang.Field)

    def patch(self, statement: StatementData, target_cks: dict[UUID, UUID]) -> None:
        statement.reference_ck = target_cks.get(statement.reference_ck, statement.reference_ck)
        statement.text = patch_text_html(statement.text, target_cks)


@dataclass
class FieldData(NodeData, HasOrder, HasCrud):
    name: Optional[str]
    key: str
    tag: TypeTag
    hint: Optional[TypeHint]
    text: Optional[str]
    flags: TypeFlag
    reference_ck: Optional[UUID] = None
    metadata: Optional[typing.Any] = None

    def __str__(self):
        name_str = f"{self.name} " if self.name else ""
        return f"{self.parent_id}:{self.order_key} {name_str}{self.tag.value}"

    def __repr__(self):
        return f"<Field {str(self)}>"


@node_packer(MNT.Field, FieldData, lang.Field)
class FieldPacker(NodePacker[FieldData, lang.Field]):
    PARENTS: ClassVar[ParentsT] = {MNT.Statement}

    def pack(self, field: lang.Field) -> "FieldData":
        reference = field.reference.ck if isinstance(field.reference, lang.Statement) else None
        return FieldData(
            id=field.id,
            ck=field.ck,
            parent_id=field.parent_id,
            order_key=field.order_key,
            name=field.name,
            key=field.key,
            tag=field.tag,
            hint=field.hint,
            flags=field.flags,
            text=field.text,
            reference_ck=reference,
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
            ck=field.ck,
            name=field.name,
            key=field.key,
            tag=field.tag,
            hint=field.hint,
            flags=field.flags,
            text=field.text,
            reference=field.reference_ck,
            metadata=field.metadata,
            revision=field.revision,
            created_at=field.created_at,
            updated_at=field.updated_at,
            last_edited_at=field.last_edited_at,
            last_changed_at=field.last_changed_at,
            _session=session,
        )

    def patch(self, node: FieldData, target_cks: dict[UUID, UUID]) -> None:
        node.reference_ck = target_cks.get(node.reference_ck, node.reference_ck)
        node.text = patch_text_html(node.text, target_cks)


@dataclass
class TriggerData(NodeData, HasCrud):
    type: TriggerType
    active: bool
    mapping: Optional[list[tuple[str, str]]]
    schedule_type: Optional[ScheduleType]
    timezone: Optional[str]
    interval: Optional[int]
    cron: Optional[str]
    runnable_ck: Optional[UUID]
    scope_ck: Optional[UUID]


@node_packer(MNT.Trigger, TriggerData, lang.Trigger)
class TriggerPacker(NodePacker[TriggerData, lang.Trigger]):
    PARENTS: ClassVar[ParentsT] = {MNT.Statement}

    def pack(self, trigger: lang.Trigger) -> "TriggerData":
        return TriggerData(
            id=trigger.id,
            ck=trigger.ck,
            parent_id=trigger.parent_id,
            type=trigger.type,
            active=trigger.active,
            mapping=trigger.mapping,
            schedule_type=trigger.schedule_type,
            timezone=trigger.timezone,
            interval=trigger.interval,
            cron=trigger.cron,
            runnable_ck=trigger.runnable.id if trigger.runnable else None,
            scope_ck=trigger.scope.id if trigger.scope else None,
            revision=trigger.revision,
            created_at=trigger.created_at,
            updated_at=trigger.updated_at,
            last_edited_at=trigger.last_edited_at,
            last_changed_at=trigger.last_changed_at,
        )

    def unpack(
        self, trigger: TriggerData, parent: lang.Statement, session: Optional[Session]
    ) -> lang.Trigger:
        return lang.Trigger(
            parent=parent,
            id=trigger.id,
            ck=trigger.ck,
            type=trigger.type,
            active=trigger.active,
            mapping=trigger.mapping,
            schedule_type=trigger.schedule_type,
            timezone=trigger.timezone,
            interval=trigger.interval,
            cron=trigger.cron,
            runnable=trigger.runnable_ck,
            scope=trigger.scope_ck,
            revision=trigger.revision,
            created_at=trigger.created_at,
            updated_at=trigger.updated_at,
            last_edited_at=trigger.last_edited_at,
            last_changed_at=trigger.last_changed_at,
            _session=session,
        )

    def patch(self, node: TriggerData, target_cks: dict[UUID, UUID]) -> None:
        node.runnable_ck = target_cks.get(node.runnable_ck, node.runnable_ck)
        node.scope_ck = target_cks.get(node.scope_ck, node.scope_ck)


@dataclass
class TaggingData(NodeData, HasCrud):
    reference_ck: Optional[UUID]
    key: str
    metadata: Optional[typing.Any] = None

    def __str__(self):
        return self.key

    def __repr__(self):
        return f"<Tagging {self}>"


@node_packer(MNT.Tagging, TaggingData, lang.Tagging)
class TaggingPacker(NodePacker[TaggingData, lang.Tagging]):
    PARENTS: ClassVar[ParentsT] = {MNT.File, MNT.Statement}

    def pack(self, tagging: lang.Tagging) -> "TaggingData":
        reference = tagging.reference.ck if isinstance(tagging.reference, lang.Statement) else None
        return TaggingData(
            id=tagging.id,
            ck=tagging.ck,
            parent_id=tagging.parent_id,
            reference_ck=reference,
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
            ck=tagging.ck,
            key=tagging.key,
            reference=tagging.reference_ck,
            metadata=tagging.metadata,
            revision=tagging.revision,
            created_at=tagging.created_at,
            updated_at=tagging.updated_at,
            last_edited_at=tagging.last_edited_at,
            last_changed_at=tagging.last_changed_at,
            _session=session,
        )

    def patch(self, node: TaggingData, target_cks: dict[UUID, UUID]) -> None:
        node.reference_ck = target_cks.get(node.reference_ck, node.reference_ck)


@dataclass
class DatasetViewData(NodeData, HasOrder, HasCrud):
    PARENTS: ClassVar[ParentsT] = {MNT.File, MNT.Statement}

    id: UUID
    name: str
    query: Optional[Query] = None
    sort: Optional[list[Sort]] = None
    length: Optional[int] = None


@node_packer(MNT.DatasetView, DatasetViewData, lang.DatasetView)
class DatasetViewPacker(NodePacker[DatasetViewData, lang.DatasetView]):
    PARENTS: ClassVar[ParentsT] = {MNT.Statement}

    def pack(self, view: lang.DatasetView) -> "DatasetViewData":
        return DatasetViewData(
            id=view.id,
            ck=view.ck,
            order_key=view.order_key,
            name=view.name,
            query=view.query,
            sort=view.sort,
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
            ck=view.ck,
            name=view.name,
            source=parent,
            query=view.query,
            sort=view.sort,
            reference=view.reference_ck,
            revision=view.revision,
            created_at=view.created_at,
            updated_at=view.updated_at,
            last_edited_at=view.last_edited_at,
            last_changed_at=view.last_changed_at,
            _session=session,
        )


@dataclass
class RecordData(NodeData, HasCrud):
    PARENTS: ClassVar[ParentsT] = {MNT.Statement}

    value: Optional[typing.Any] = None

    def __str__(self):
        return f"{self.parent_id} {describe_type(self.value)}"

    def __repr__(self):
        return f"<{self.__class__.__name__} {str(self)}>"


@node_packer(MNT.Record, RecordData, lang.Record)
class RecordPacker(NodePacker[RecordData, lang.Record]):
    PARENTS: ClassVar[ParentsT] = {MNT.Statement}

    def pack(self, record: lang.Record) -> "RecordData":
        return RecordData(
            id=record.id,
            ck=record.ck,
            parent_id=record.parent_id,
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
            ck=record.ck,
            parent=parent,
            value=record.value,
            revision=record.revision,
            created_at=record.created_at,
            updated_at=record.updated_at,
            last_edited_at=record.last_edited_at,
            last_changed_at=record.last_changed_at,
            _instantiated=False,
        )


@dataclass
class ResolvedFieldData(NodeData):
    field_ck: UUID


@node_packer(MNT.ResolvedField, ResolvedFieldData, lang.ResolvedField)
class ResolvedFieldPacker(NodePacker[ResolvedFieldData, lang.ResolvedField]):
    PARENTS: ClassVar[ParentsT] = {MNT.Statement}

    def pack(self, resolved_field: lang.ResolvedField) -> "ResolvedFieldData":
        return ResolvedFieldData(
            id=resolved_field.id,
            ck=resolved_field.ck,
            parent_id=resolved_field.parent_id,
            field_ck=resolved_field.field_ck,
        )

    def patch(self, node: ResolvedFieldData, target_cks: dict[UUID, UUID]) -> None:
        node.field_ck = target_cks.get(node.field_ck, node.field_ck)


@dataclass
class IssueData(NodeData):
    kind: IssueKind
    type: IssueType
    message: Optional[str]


@node_packer(MNT.Issue, IssueData, lang.Issue)
class IssuePacker(NodePacker[IssueData, lang.Issue]):
    PARENTS: ClassVar[ParentsT] = {MNT.Statement}

    def pack(self, issue: lang.Issue) -> "IssueData":
        return IssueData(
            id=issue.id,
            ck=issue.ck,
            parent_id=issue.subject_id,
            kind=issue.kind,
            type=issue.type,
            message=issue.message,
        )


# other objects


class DataPacker(abc.ABC, typing.Generic[DataT, ObjectT]):
    """Generic data packer for non-node module data types"""

    def pack(self, object: ObjectT) -> DataT:
        raise NotImplementedError

    def unpack(self, data: DataT, module: Module) -> ObjectT:
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


def unpack_data(data: DataT, module: Module) -> ObjectT:
    """Unpack a flat module node into a language data object"""
    packer = _data_packers_by_data[type(data)]
    return packer.unpack(data, module)


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

    def unpack(self, data: RemoteObjectData, module: Module) -> lang.RemoteObject:
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

    def unpack(self, data: SecretData, module: Module) -> lang.Secret:
        return lang.Secret(id=data.id, sha512=data.sha512, value=data.value)


@dataclass
class SessionData:
    id: UUID
    module_id: UUID
    opened_at: Optional[datetime]
    closed_at: Optional[datetime]
    metadata: Optional[dict[str, Any]]
    trigger_id: Optional[UUID]
    trigger_type: TriggerType


@data_packer(SessionData, lang.Session)
class SessionPacker(DataPacker[SessionData, lang.Session]):
    def pack(self, object: lang.Session) -> SessionData:
        return SessionData(
            id=object.id,
            module_id=object.module.id,
            opened_at=object.opened_at,
            closed_at=object.closed_at,
            metadata=object.metadata,
            trigger_id=object.ctx.trigger_id,
            trigger_type=object.ctx.trigger_type,
        )


@dataclass
class RunErrorData:
    kind: RunErrorKind
    type: str
    message: Optional[str]
    runnable_id: Optional[UUID]
    traceback: list[RunCodeFrame]

    @staticmethod
    def from_dict(data: dict[str, Any]) -> "RunErrorData":
        return RunErrorData(
            kind=RunErrorKind(data["kind"]),
            type=data["type"],
            message=data["message"],
            runnable_id=data["runnable_id"],
            traceback=[RunCodeFrame.from_dict(frame) for frame in data["traceback"]],
        )

    def to_dict(self) -> dict[str, Any]:
        return {
            "kind": self.kind.value,
            "type": self.type,
            "message": self.message,
            "runnable_id": str(self.runnable_id),
            "traceback": [dataclasses.asdict(frame) for frame in self.traceback],
        }


@dataclass
class RunData:
    id: UUID
    worker_node_id: Optional[str]
    worker_process_id: Optional[str]
    project_id: UUID
    module_id: UUID
    runnable_id: UUID
    runnable_ck: UUID
    runnable_type: StatementType
    session_id: Optional[UUID]
    trigger_type: TriggerType
    trigger_id: Optional[UUID]
    root_id: Optional[UUID]
    parent_id: Optional[UUID]
    created_at: datetime
    updated_at: datetime
    scheduled_at: Optional[datetime]
    started_at: Optional[datetime]
    terminated_at: Optional[datetime]
    status: RunStatus
    inputs: Optional[Any]
    outputs: Optional[Any]
    error: Optional[RunErrorData]
    metadata: Optional[dict[str, Any]]


@data_packer(RunData, bench.language.run.Run)
class RunPacker(DataPacker[RunData, bench.language.run.Run]):
    def pack(self, object: bench.language.run.Run) -> RunData:
        if object.error:
            error = RunErrorData(
                kind=object.error.kind,
                type=object.error.type,
                message=object.error.message,
                runnable_id=object.runnable.id,
                traceback=object.error.traceback,
            )
        else:
            error = None
        trigger_id = (
            object.trigger
            if isinstance(object.trigger, UUID)
            else object.trigger.id
            if object.trigger
            else None
        )
        return RunData(
            id=object.id,
            project_id=object.session.ctx.project_id,
            module_id=object.session.module.id,
            worker_node_id=object.session.ctx.worker_node_id,
            worker_process_id=object.session.ctx.worker_process_id,
            runnable_id=object.runnable.id,
            runnable_ck=object.runnable.ck,
            runnable_type=object.runnable.type,
            session_id=object.session.id,
            trigger_id=trigger_id,
            trigger_type=object.trigger_type if object.trigger_type else None,
            root_id=object.root.id if object.root else None,
            parent_id=object.parent.id if object.parent else None,
            created_at=object.created_at,
            updated_at=object.updated_at,
            scheduled_at=object.scheduled_at,
            started_at=object.started_at,
            terminated_at=object.terminated_at,
            status=object.status,
            inputs=object.inputs,
            outputs=object.outputs,
            error=error,
            metadata=object.metadata,
        )

    def unpack(self, data: RunData, module: Module) -> bench.language.run.Run:
        # we leave relational references that aren't in the module as None?
        runnable = module._nodes_by_ck.get(data.runnable_ck) or MissingStatement(
            data.runnable_id, data.runnable_type
        )
        if data.error:
            error = bench.language.run.RunError(
                kind=data.error.kind,
                type=data.error.type,
                message=data.error.message,
                traceback=data.error.traceback,
                runnable=runnable,
            )
        else:
            error = None
        return bench.language.run.Run(
            id=data.id,
            module=module,
            session=None,
            root=LazyRun(data.root_id) if data.root_id else None,
            parent=LazyRun(data.parent_id) if data.parent_id else None,
            runnable=runnable,
            trigger=data.trigger_id,
            trigger_type=data.trigger_type,
            created_at=data.created_at,
            updated_at=data.updated_at,
            scheduled_at=data.scheduled_at,
            started_at=data.started_at,
            terminated_at=data.terminated_at,
            inputs=data.inputs,
            outputs=data.outputs,
            error=error,
            metadata=data.metadata,
        )


@dataclass
class LogEntryData:
    id: UUID
    module_id: UUID
    created_at: datetime
    stream: str
    level: Optional[str]
    logger: Optional[str]
    message: Optional[str]
    session_id: Optional[UUID]
    runnable_id: Optional[UUID]
    runnable_ck: Optional[UUID]
    run_id: Optional[UUID]
    metadata: Optional[dict[str, Any]]


@data_packer(LogEntryData, lang.LogEntry)
class LogEntryPacker(DataPacker[LogEntryData, lang.LogEntry]):
    def pack(self, object: lang.LogEntry) -> LogEntryData:
        return LogEntryData(
            id=object.id,
            module_id=object.module.id,
            created_at=object.created_at,
            stream=object.stream,
            level=object.level,
            logger=object.logger,
            message=object.message,
            session_id=object.session.id,
            runnable_id=object.runnable.id if object.runnable else None,
            runnable_ck=object.runnable.ck if object.runnable else None,
            run_id=object.run.id if object.run else None,
            metadata=object.metadata,
        )

    def unpack(self, data: LogEntryData, module: Module) -> lang.LogEntry:
        runnable = module._nodes_by_id.get(data.runnable_id) or MissingStatement(data.runnable_id)
        run = LazyRun(data.run_id) if data.run_id else None
        return lang.LogEntry(
            id=data.id,
            module=module,
            session=None,
            created_at=data.created_at,
            stream=data.stream,
            level=data.level,
            logger=data.logger,
            message=data.message,
            runnable=runnable,
            run=run,
            metadata=data.metadata,
        )


@dataclass
class WorkerSetData:
    id: UUID
    project_id: UUID
    region: WorkerRegion
    profile: WorkerProfile
    sleeping: bool
    status: WorkerSetStatus
    desired_replicas: int
    target_replicas: int
    available_replicas: int
    ready_replicas: int
    created_at: datetime
    updated_at: datetime
    last_active_at: Optional[datetime]


@dataclass
class EnvironmentData:
    language: str
    version: str
    platform: str
    packages: dict[str, str]


def serialize_module(module_data: ModuleTreeData) -> bytes:
    module_data_dict = to_dict(module_data, omit_empty=True)
    module_data = msgpack.packb(module_data_dict, use_bin_type=True)
    return module_data


def deserialize_module(module_data: bytes) -> ModuleTreeData:
    module_data_dict = msgpack.unpackb(module_data, raw=False)
    module_data = from_dict(ModuleTreeData, module_data_dict)
    return module_data


@dataclass
class MissingStatement:
    id: UUID
    type: Optional[StatementType] = None

    def __str__(self):
        return str(self.id)

    def __repr__(self):
        return f"<MissingStatement {self.id} {self.type or '<unknown type>'}>"

    def __getattr__(self, item):
        if item == "id":
            return self.id
        raise AttributeError(f"statement {self.id} not found, so {item} cannot be accessed")
