from __future__ import annotations

import abc
import dataclasses
import typing
from collections import OrderedDict
from dataclasses import dataclass, replace
from datetime import datetime
from typing import Any, ClassVar, Optional
from uuid import UUID

import msgpack

from bench import language as lang
from bench.language import File, IssueType, Module, NodeVisitor
from bench.language.const import (
    MNT,
    BlobStatus,
    IssueKind,
    ModuleNodeType,
    NodeTrackingLevel,
    RunStatus,
    ScheduleType,
    SessionAccessLevel,
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
from bench.language.database import HasDatabase
from bench.language.module import Node, NodeStatus, NodeTree, ScopeNode
from bench.language.query import Query, Sort
from bench.language.run import Run, RunCodeFrame, RunError, RunErrorKind
from bench.language.session import LazyRun, Session
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
NodeT = typing.TypeVar("NodeT", bound=Node)
DataT = typing.TypeVar("DataT")
ObjectT = typing.TypeVar("ObjectT")


class NodePacker(abc.ABC, typing.Generic[NodeDataT, NodeT]):
    """Module node packer"""

    mnt: ClassVar[MNT]
    PARENTS: ClassVar[ParentsT]
    REMAP: ClassVar[dict[str, str]] = {}

    def pack(self, node: NodeT) -> NodeDataT:
        """Packs the node itself into the wire format"""
        raise NotImplementedError(f"pack not supported in {self.__class__.__name__}")

    def unpack(self, node: NodeDataT, parent: Optional[NodeT], session: Optional[Session]) -> NodeT:
        """Unpacks the node itself from the wire format (plain or instrumented into session)"""
        raise NotImplementedError(f"unpack not supported in {self.__class__.__name__}")

    def patch(
        self, node: NodeDataT, target_cks: dict[UUID, UUID], target_keys: dict[str, str]
    ) -> None:
        pass


class PackContext(NodeVisitor):
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
        _node_packers_by_node[node_t] = packer
        _node_packers_by_data[data_t] = packer
        MNT_BY_DATA_CLASS[data_t] = mnt
        DATA_CLASS_BY_MNT[mnt] = data_t
        return cls

    return decorator


def pack_module(module: Module, exclude: set[MNT] | None = None) -> "ModuleTreeData":
    module_data, nodes = pack_node(module, exclude=exclude)
    module_tree = ModuleTreeData(**module_data.__dict__, module=module_data, nodes=nodes)
    return module_tree


def unpack_module(
    nodes: list[NodeDataT], session: Optional[Session], exclude: set[MNT] | None = None
) -> Module:
    module = unpack_node(NodeTree(nodes), parent=None, session=session, exclude=exclude)
    return module


def pack_node(root: NodeT, exclude: set[MNT] | None = None) -> tuple[NodeDataT, list[NodeDataT]]:
    """Pack a node and all its descendants"""
    exclude = exclude or tuple()
    packed: dict[UUID, NodeDataT] = OrderedDict()

    # walk and pack until nothing is left to pack
    to_pack = root._local_root_tree.get_descendants(root.ck, include_self=True, recursive=True)
    while to_pack:
        next_pack = []
        for node in to_pack:
            if node.mnt in exclude:
                continue
            packer = _node_packers_by_node[type(node)]
            packed[node.id] = packer.pack(node)

            # records are not part of regular node tree :NodeViews
            if (
                node.mnt == MNT.STATEMENT
                and HasDatabase in node._components
                and node.versioned
                and MNT.RECORD not in exclude
            ):
                next_pack.extend(node.records)
        to_pack = next_pack

    return packed[root.id], list(packed.values())


def unpack_node(
    data_tree: NodeTree[NodeDataT],
    parent: Optional[NodeT],
    session: Optional[Session],
    exclude: set[MNT] | None = None,
) -> NodeT:
    """Unpack a node and all its descendants and index them"""
    exclude = exclude or tuple()
    unpacked_tree = NodeTree()

    # unpack all nodes top down (breadth first)
    for data_node in data_tree.walk_bfs():
        if data_node.mnt in exclude:
            continue

        packer = _node_packers_by_data[type(data_node)]
        if data_node.parent_id is None:
            node_parent = parent
        elif data_node.parent_id not in unpacked_tree.nodes_by_id:
            if parent is not None and data_node.parent_id == parent.id:
                node_parent = parent
            else:
                raise ValueError(
                    f"node {data_node!r} parent {data_node.parent_id} not found in unpacked {unpacked_tree!r}"
                )
        else:
            node_parent = unpacked_tree.nodes_by_id[data_node.parent_id]
        node = packer.unpack(data_node, node_parent, session)

        # keep parent instance if it was passed (update in place)
        if parent is not None and node.id == parent.id:
            for prop in parent.__properties__.values():
                if not prop.is_runtime and not prop.is_relation:
                    setattr(parent, prop.name, getattr(node, prop.name))
            node = parent

        unpacked_tree.add(node)

    # index & recover node lists
    root = unpacked_tree.root
    if isinstance(root, ScopeNode):
        root._local_root_tree.set(unpacked_tree.nodes_by_ck.values())
    for node in unpacked_tree.nodes_by_id.values():
        node._status = NodeStatus.SOURCE  # status is auto-set to interpreted if a session is active
        if isinstance(node, ScopeNode):
            node._update_lists(node)

    if isinstance(root, ScopeNode):
        root._index_rec()
    elif isinstance(root, Node):
        root._index_self()
    else:
        raise ValueError(f"unexpected root {root} ({type(root)})")

    return root


def pack_node_flat(node: NodeT) -> NodeDataT:
    """Pack a language node into a flat module node"""
    packer = _node_packers_by_node[type(node)]
    return packer.pack(node)


def unpack_node_flat(node: NodeDataT, parent: Optional[NodeT], session: Optional[Session]) -> NodeT:
    """Unpack a flat module node into a language node"""
    packer = _node_packers_by_data[type(node)]
    return packer.unpack(node, parent, session)


def patch_node_flat(
    node: NodeDataT, target_cks: dict[UUID, UUID], target_keys: dict[str, str]
) -> NodeDataT:
    """Patch a flat module node"""
    packer = _node_packers_by_data[type(node)]
    packer.patch(node, target_cks, target_keys)
    return node


def remap_properties(mnt: MNT, properties: list[str] | None) -> list[str] | None:
    if properties is None:
        return None
    packer = _node_packers_by_data[DATA_CLASS_BY_MNT[mnt]]
    properties = [packer.REMAP.get(p, p) for p in properties]
    return properties


@dataclass
class NodeData:
    mnt: ClassVar[MNT]  # not great but wire data will be refactored anyway
    id: UUID
    ck: UUID
    parent_id: Optional[UUID]

    @property
    def mnt(self) -> ModuleNodeType:
        return MNT_BY_DATA_CLASS[type(self)]

    def equals_content(self, other: "NodeData") -> bool:
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
    mnt: ClassVar[MNT] = MNT.MODULE
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


@node_packer(MNT.MODULE, ModuleData, Module)
class ModulePacker(NodePacker[ModuleData, Module]):
    PARENTS: ClassVar[ParentsT] = set()
    REMAP: ClassVar[dict[str, str]] = {}

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
            _status=NodeStatus.SOURCE,
        )


@dataclass
class FileData(NodeData, HasCrud):
    name: str

    def __str__(self):
        return f"{self.name}"

    def __repr__(self):
        return f"<File {str(self)}>"


@node_packer(MNT.FILE, FileData, File)
class FilePacker(NodePacker[FileData, File]):
    mnt: ClassVar[MNT] = MNT.FILE
    PARENTS: ClassVar[ParentsT] = {MNT.MODULE}
    REMAP: ClassVar[dict[str, str]] = {}

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
            name=file.name,
            parent=parent,
            revision=file.revision,
            created_at=file.created_at,
            updated_at=file.updated_at,
            last_edited_at=file.last_edited_at,
            last_changed_at=file.last_changed_at,
            _session=session,
            _status=NodeStatus.SOURCE,
        )


@dataclass
class StatementData(NodeData, HasOrder, HasCrud):
    mnt: ClassVar[MNT] = MNT.STATEMENT
    type: StatementType
    name: Optional[str]
    heading_level: Optional[TextHeadingLevel]
    text: Optional[str]
    key: Optional[str]
    code: Optional[str]
    value: Optional[typing.Any]
    versioned: Optional[bool]
    reference_ck: Optional[UUID]

    def __str__(self):
        return f"{self.type.name} {self.name}"

    def __repr__(self):
        return f"<{self.__class__.__name__} {str(self)}>"


@node_packer(MNT.STATEMENT, StatementData, lang.Statement)
class StatementPacker(NodePacker[StatementData, lang.Statement]):
    PARENTS: ClassVar[ParentsT] = {MNT.STATEMENT, MNT.FILE}
    REMAP: ClassVar[dict[str, str]] = {"reference": "reference_ck"}

    def pack(self, statement: lang.Statement) -> "StatementData":
        value = (
            statement._raw_value() if lang.HasValue in statement._components else statement.value
        )
        return StatementData(
            id=statement.id,
            ck=statement.ck,
            parent_id=statement.parent_id,
            order_key=statement.order_key,
            type=statement.type,
            name=statement.name,
            heading_level=statement.heading_level,
            text=statement.text,
            key=statement.key,
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
        parent: lang.File | lang.Statement,
        session: Optional[Session],
    ) -> lang.Statement:
        return lang.Statement(
            id=statement.id,
            ck=statement.ck,
            parent=parent,
            order_key=statement.order_key,
            type=statement.type,
            name=statement.name,
            heading_level=statement.heading_level,
            text=statement.text,
            key=statement.key,
            code=statement.code,
            value=statement.value,
            versioned=statement.versioned,
            reference=statement.reference_ck,
            revision=statement.revision,
            created_at=statement.created_at,
            updated_at=statement.updated_at,
            last_edited_at=statement.last_edited_at,
            last_changed_at=statement.last_changed_at,
            _status=NodeStatus.SOURCE,
            _session=session,
        )

    def patch(
        self, statement: StatementData, target_cks: dict[UUID, UUID], target_keys: dict[str, str]
    ) -> None:
        statement.reference_ck = target_cks.get(statement.reference_ck, statement.reference_ck)
        statement.key = target_keys.get(statement.key, statement.key)
        statement.text = patch_text_html(statement.text, target_cks)


@dataclass
class FieldData(NodeData, HasOrder, HasCrud):
    mnt: ClassVar[MNT] = MNT.FIELD
    name: Optional[str]
    key: str
    tag: TypeTag
    hint: Optional[TypeHint]
    text: Optional[str]
    flags: TypeFlag
    reference_ck: Optional[UUID] = None
    value: Optional[typing.Any] = None

    def __str__(self):
        name_str = f"{self.name} " if self.name else ""
        return f"{self.parent_id}:{self.order_key} {name_str}{self.tag.value}"

    def __repr__(self):
        return f"<Field {str(self)}>"


@node_packer(MNT.FIELD, FieldData, lang.Field)
class FieldPacker(NodePacker[FieldData, lang.Field]):
    PARENTS: ClassVar[ParentsT] = {MNT.STATEMENT}
    REMAP: ClassVar[dict[str, str]] = {"reference": "reference_ck"}

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
            value=field._raw_value(),
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
            order_key=field.order_key,
            name=field.name,
            key=field.key,
            tag=field.tag,
            hint=field.hint,
            flags=field.flags,
            text=field.text,
            reference=field.reference_ck,
            value=field.value,
            revision=field.revision,
            created_at=field.created_at,
            updated_at=field.updated_at,
            last_edited_at=field.last_edited_at,
            last_changed_at=field.last_changed_at,
            _status=NodeStatus.SOURCE,
            _session=session,
        )

    def patch(
        self, node: FieldData, target_cks: dict[UUID, UUID], target_keys: dict[str, str]
    ) -> None:
        node.reference_ck = target_cks.get(node.reference_ck, node.reference_ck)
        node.key = target_keys.get(node.key, node.key)
        node.text = patch_text_html(node.text, target_cks)


@dataclass
class ResolvedFieldData(NodeData):
    field_ck: UUID


@node_packer(MNT.RESOLVED_FIELD, ResolvedFieldData, lang.ResolvedField)
class ResolvedFieldPacker(NodePacker[ResolvedFieldData, lang.ResolvedField]):
    mnt: ClassVar[MNT] = MNT.RESOLVED_FIELD
    PARENTS: ClassVar[ParentsT] = {MNT.STATEMENT}

    def pack(self, resolved_field: lang.ResolvedField) -> "ResolvedFieldData":
        FieldPacker.pack(self, resolved_field)
        return ResolvedFieldData(
            id=resolved_field.id,
            ck=resolved_field.ck,
            parent_id=resolved_field.parent_id,
            field_ck=resolved_field.field_ck,
        )

    def patch(
        self, node: ResolvedFieldData, target_cks: dict[UUID, UUID], target_keys: dict[str, str]
    ) -> None:
        node.field_ck = target_cks.get(node.field_ck, node.field_ck)


@dataclass
class TriggerData(NodeData, HasCrud):
    mnt: ClassVar[MNT] = MNT.TRIGGER
    type: TriggerType
    active: bool
    mapping: Optional[list[tuple[str, str]]]
    schedule_type: Optional[ScheduleType]
    timezone: Optional[str]
    interval: Optional[int]
    cron: Optional[str]
    statement_ck: Optional[UUID]
    scope_ck: Optional[UUID]


@node_packer(MNT.TRIGGER, TriggerData, lang.Trigger)
class TriggerPacker(NodePacker[TriggerData, lang.Trigger]):
    PARENTS: ClassVar[ParentsT] = {MNT.STATEMENT}
    REMAP: ClassVar[dict[str, str]] = {"statement": "statement_ck", "scope": "scope_ck"}

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
            statement_ck=trigger.statement.ck if trigger.statement else None,
            scope_ck=trigger.scope.ck if trigger.scope else None,
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
            statement=trigger.statement_ck,
            scope=trigger.scope_ck,
            revision=trigger.revision,
            created_at=trigger.created_at,
            updated_at=trigger.updated_at,
            last_edited_at=trigger.last_edited_at,
            last_changed_at=trigger.last_changed_at,
            _status=NodeStatus.SOURCE,
            _session=session,
        )

    def patch(
        self, node: TriggerData, target_cks: dict[UUID, UUID], target_keys: dict[str, str]
    ) -> None:
        node.statement_ck = target_cks.get(node.statement_ck, node.statement_ck)
        node.scope_ck = target_cks.get(node.scope_ck, node.scope_ck)


@dataclass
class TaggingData(NodeData, HasCrud):
    mnt: ClassVar[MNT] = MNT.TAGGING
    reference_ck: Optional[UUID]
    key: str
    value: Optional[typing.Any] = None

    def __str__(self):
        return self.key

    def __repr__(self):
        return f"<Tagging {self}>"


@node_packer(MNT.TAGGING, TaggingData, lang.Tagging)
class TaggingPacker(NodePacker[TaggingData, lang.Tagging]):
    PARENTS: ClassVar[ParentsT] = {MNT.FILE, MNT.STATEMENT}
    REMAP: ClassVar[dict[str, str]] = {"reference": "reference_ck"}

    def pack(self, tagging: lang.Tagging) -> "TaggingData":
        reference = tagging.reference.ck if isinstance(tagging.reference, lang.Statement) else None
        return TaggingData(
            id=tagging.id,
            ck=tagging.ck,
            parent_id=tagging.parent_id,
            reference_ck=reference,
            key=tagging.key,
            value=tagging._raw_value(),
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
            value=tagging.value,
            revision=tagging.revision,
            created_at=tagging.created_at,
            updated_at=tagging.updated_at,
            last_edited_at=tagging.last_edited_at,
            last_changed_at=tagging.last_changed_at,
            _status=NodeStatus.SOURCE,
            _session=session,
        )

    def patch(
        self, node: TaggingData, target_cks: dict[UUID, UUID], target_keys: dict[str, str]
    ) -> None:
        node.reference_ck = target_cks.get(node.reference_ck, node.reference_ck)


@dataclass
class DatabaseViewData(NodeData, HasOrder, HasCrud):
    mnt: ClassVar[MNT] = MNT.DATABASE_VIEW
    PARENTS: ClassVar[ParentsT] = {MNT.FILE, MNT.STATEMENT}

    id: UUID
    name: str
    query: Optional[Query] = None
    sort: Optional[list[Sort]] = None


@node_packer(MNT.DATABASE_VIEW, DatabaseViewData, lang.DatabaseView)
class DatabaseViewPacker(NodePacker[DatabaseViewData, lang.DatabaseView]):
    PARENTS: ClassVar[ParentsT] = {MNT.STATEMENT}
    REMAP: ClassVar[dict[str, str]] = {}

    def pack(self, view: lang.DatabaseView) -> "DatabaseViewData":
        return DatabaseViewData(
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
        self, view: DatabaseViewData, parent: lang.Statement, session: Optional[Session]
    ) -> lang.DatabaseView:
        return lang.DatabaseView(
            id=view.id,
            ck=view.ck,
            name=view.name,
            source=parent,
            query=view.query,
            sort=view.sort,
            revision=view.revision,
            created_at=view.created_at,
            updated_at=view.updated_at,
            last_edited_at=view.last_edited_at,
            last_changed_at=view.last_changed_at,
            _status=NodeStatus.SOURCE,
            _session=session,
        )


@dataclass
class RecordData(NodeData, HasCrud):
    mnt: ClassVar[MNT] = MNT.RECORD
    PARENTS: ClassVar[ParentsT] = {MNT.STATEMENT}

    parent_key: str
    value: Optional[typing.Any] = None

    def __str__(self):
        return f"{self.parent_id} {describe_type(self.value) or '<empty>'}"

    def __repr__(self):
        return f"<{self.__class__.__name__} {str(self)}>"


@node_packer(MNT.RECORD, RecordData, lang.Record)
class RecordPacker(NodePacker[RecordData, lang.Record]):
    PARENTS: ClassVar[ParentsT] = {MNT.STATEMENT}

    def pack(self, record: lang.Record) -> "RecordData":
        return RecordData(
            id=record.id,
            ck=record.ck,
            parent_id=record.parent_id,
            parent_key=record.parent.key,
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
            _status=NodeStatus.SOURCE,
            _session=session,
        )


@dataclass
class IssueData(NodeData):
    kind: IssueKind
    type: IssueType
    message: Optional[str]


@node_packer(MNT.ISSUE, IssueData, lang.Issue)
class IssuePacker(NodePacker[IssueData, lang.Issue]):
    mnt: ClassVar[MNT] = MNT.ISSUE
    PARENTS: ClassVar[ParentsT] = {MNT.STATEMENT}

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
class BlobData:
    id: UUID
    sha512: str
    content_length: int
    content_type: str
    name: Optional[str]
    status: BlobStatus

    def __str__(self):
        return f"{self.id} {self.name} ({self.content_type}, {self.content_length} bytes)"

    def __repr__(self):
        return f"<Blob {self}>"


@data_packer(BlobData, lang.Blob)
class BlobPacker(DataPacker[BlobData, lang.Blob]):
    def pack(self, object: lang.Blob) -> BlobData:
        return BlobData(
            id=object.id,
            sha512=object.sha512,
            content_length=object.content_length,
            content_type=object.content_type,
            name=object.name,
            status=object.status,
        )

    def unpack(self, data: BlobData, module: Module) -> lang.Blob:
        return lang.Blob(
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
    trigger_id: Optional[UUID]
    trigger_type: TriggerType


@data_packer(SessionData, lang.Session)
class SessionPacker(DataPacker[SessionData, lang.Session]):
    def pack(self, session: lang.Session) -> SessionData:
        return SessionData(
            id=session.id,
            module_id=session.module.id,
            opened_at=session.opened_at,
            closed_at=session.closed_at,
            trigger_id=session.trigger_id,
            trigger_type=session.trigger_type,
        )


@dataclass
class RunErrorData:
    kind: RunErrorKind
    type: str
    message: Optional[str]
    statement_id: Optional[UUID]
    traceback: list[RunCodeFrame]

    @staticmethod
    def from_dict(data: dict[str, Any]) -> "RunErrorData":
        return RunErrorData(
            kind=RunErrorKind(data["kind"]),
            type=data["type"],
            message=data["message"],
            statement_id=data["statement_id"],
            traceback=[RunCodeFrame.from_dict(frame) for frame in data["traceback"]],
        )

    def to_dict(self) -> dict[str, Any]:
        return {
            "kind": self.kind.value,
            "type": self.type,
            "message": self.message,
            "statement_id": str(self.statement_id),
            "traceback": [dataclasses.asdict(frame) for frame in self.traceback or []],
        }


@dataclass
class RunData:
    id: UUID
    worker_node_id: Optional[str]
    worker_process_id: Optional[str]
    project_id: UUID
    module_id: UUID
    statement_id: Optional[UUID]
    statement_ck: Optional[UUID]
    statement_type: StatementType
    statement_path: Optional[str]
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
    value: Optional[dict[str, Any]]
    access_level: Optional[SessionAccessLevel]


@data_packer(RunData, Run)
class RunPacker(DataPacker[RunData, Run]):
    def pack(self, run: Run) -> RunData:
        track_statement = run.statement._track >= NodeTrackingLevel.FULL
        if run.error:
            error = RunErrorData(
                kind=run.error.kind,
                type=run.error.type,
                message=run.error.message,
                statement_id=run.statement.id if track_statement else None,
                traceback=run.error.traceback,
            )
        else:
            error = None
        trigger_id = (
            run.trigger
            if isinstance(run.trigger, UUID)
            else run.trigger.id
            if run.trigger
            else None
        )
        return RunData(
            id=run.id,
            project_id=run.module.project_id,
            module_id=run.module.id,
            worker_node_id=run.session.worker_node_id,
            worker_process_id=run.session.worker_process_id,
            statement_id=run.statement.id if track_statement else None,
            statement_ck=run.statement.ck if track_statement else None,
            statement_type=run.statement.type if track_statement else None,
            statement_path=run.statement_path,
            session_id=run.session.id,
            trigger_id=trigger_id,
            trigger_type=run.trigger_type if run.trigger_type else None,
            root_id=run.root.id if run.root else None,
            parent_id=run.parent.id if run.parent else None,
            created_at=run.created_at,
            updated_at=run.updated_at,
            scheduled_at=run.scheduled_at,
            started_at=run.started_at,
            terminated_at=run.terminated_at,
            status=run.status,
            inputs=run.inputs,
            outputs=run.outputs,
            error=error,
            value=run._raw_value(),
            access_level=run.access_level,
        )

    def unpack(self, data: RunData, module: Module) -> Run:
        # we leave relational references that aren't in the module as None?
        if data.error:
            error = RunError(
                kind=data.error.kind,
                type=data.error.type,
                message=data.error.message,
                traceback=data.error.traceback,
                statement=None,  # obviously wrong
            )
        else:
            error = None
        return Run(
            id=data.id,
            module=module,
            session=None,
            root=LazyRun(data.root_id) if data.root_id else None,
            parent=LazyRun(data.parent_id) if data.parent_id else None,
            statement=None,  # obviously wrong
            statement_path=data.statement_path,
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
            value=data.value,
            access_level=data.access_level,
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
    statement_id: Optional[UUID]
    statement_ck: Optional[UUID]
    run_id: Optional[UUID]
    value: Optional[dict[str, Any]]


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
            statement_id=object.statement.id if object.statement else None,
            statement_ck=object.statement.ck if object.statement else None,
            run_id=object.run.id if object.run else None,
            value=object.value,
        )

    def unpack(self, data: LogEntryData, module: Module) -> lang.LogEntry:
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
            statement=None,  # obviously wrong
            run=run,
            value=data.value,
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
