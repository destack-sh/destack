from __future__ import annotations

import abc
import enum
import re
import traceback
import typing
from collections import defaultdict
from dataclasses import dataclass
from datetime import datetime
from hashlib import md5
from typing import Any, ClassVar, Optional
from uuid import UUID

from bench import bench as language
from bench.bench import Code, IssueType, StatementType, TypeHint, TypeTag
from bench.bench.const import ExecutionTriggerType, ExpectationModifier, InterpScope, TypeFlag
from bench.bench.dataset import Query, Sort
from bench.bench.issue import IssueKind
from bench.bench.type import ModuleNode, ModuleReference
from bench.runtime.common.type import ExecutionFrame, PyFrameData, RunErrorData
from bench.utils.func import describe_type

if typing.TYPE_CHECKING:
    from bench.bench.build import XBlock
    from bench.bench.session import Session


#
# Stable, concise and flat language data nodes for transit and storage.
# TODO @Performance @Robustness: use an optimized and evolvable :WireFormat
#


class ModuleObjectType(enum.StrEnum):
    MODULE = "MODULE"
    FILE = "FILE"
    STATEMENT = "STATEMENT"
    FIELD = "FIELD"
    RECORD = "RECORD"
    DATASET_VIEW = "DATASET_VIEW"
    INTERP = "INTERP"


MOT = ModuleObjectType
ParentsT = set[MOT]
NodeDataT = typing.TypeVar("NodeDataT", bound="NodeData")
NodeT = typing.TypeVar("NodeT", bound="ModuleNode")
DataT = typing.TypeVar("DataT")
ObjectT = typing.TypeVar("ObjectT")


class ModuleTree:
    """An indexed tree of module objects"""

    def __init__(self):
        self.nodes: dict[tuple[MOT, UUID], NodeData] = {}
        self.children: dict[tuple[MOT, UUID], list[NodeData]] = defaultdict(list)

    def add(self, node: NodeData):
        mot = MOT_BY_DATA_CLASS[type(node)]
        self.nodes[(mot, node.id)] = node
        if node.parent_id:
            pass

    def to_module(self) -> ModuleData:
        raise NotImplementedError


class DataPacker(typing.Generic[DataT, ObjectT]):
    """Generic data packer for non-node data types"""

    def pack(self, object: ObjectT) -> DataT:
        raise NotImplementedError

    def unpack(self, data: DataT) -> ObjectT:
        raise NotImplementedError


class NodePacker(abc.ABC, typing.Generic[NodeDataT, NodeT]):
    """Module node packer"""

    def walk(self, node: NodeT, tree: PackContext) -> None:
        """Walks all direct children of the given node"""
        pass

    def pack(self, node: NodeT) -> NodeDataT:
        """Packs the node itself into the wire format"""
        raise NotImplementedError

    def unpack(self, node: NodeDataT, parent: Optional[NodeT]) -> NodeT:
        """Unpacks the node itself from the wire format"""
        raise NotImplementedError

    def unwalk(self, node: NodeT, tree: UnpackContext) -> None:
        """Re-assigns the node's children"""
        pass


class PackContext(abc.ABC):
    """Tree visitor for packing"""

    def __init__(self):
        self.visited: dict[UUID, ModuleNode] = {}

    def visit(self, node: ModuleNode):
        self.visited[node.id] = node


class UnpackContext(abc.ABC):
    """Tree visitor for unpacking"""

    def get_one(self, parent_id: UUID, t: typing.Type[ModuleNode]) -> Optional["ModuleNode"]:
        return None

    def get_many(
        self, parent_id: UUID, t: typing.Type[ModuleNode], recursive: bool = False
    ) -> list["ModuleNode"]:
        return []


# registered packers, where each MOT may have multiple packers (subtypes)
_node_packers: dict[MOT, dict[typing.Type[NodeDataT], "NodePacker"]] = defaultdict(dict)
MOT_BY_DATA_CLASS: dict[typing.Type[NodeDataT], MOT] = {}


def node_packer(t: MOT, data_t: typing.Type[NodeDataT], node_t: typing.Type[NodeT] | None):
    """Decorator to register a node packer for a given type"""

    def decorator(cls: "NodePacker"):
        if node_t in _node_packers[t]:
            raise ValueError(
                f"packer for {t} and {node_t} already registered: {_node_packers[t][node_t]}"
            )
        if node_t in MOT_BY_DATA_CLASS:
            raise ValueError(f"data class {data_t} already registered: {MOT_BY_DATA_CLASS[data_t]}")
        _node_packers[t][node_t] = cls
        MOT_BY_DATA_CLASS[data_t] = t
        return cls

    return decorator


def pack_module(module: language.Module) -> "ModuleData":
    module_data, nodes = pack_node(module)
    module_data.nodes = nodes
    return module_data


def pack_node(root: NodeT) -> tuple[NodeDataT, list[NodeDataT]]:
    packed: dict[UUID, NodeDataT] = {}
    ctx = PackContext()

    to_pack: list[NodeT] = [root]
    while to_pack is not None:
        for node in to_pack:
            packer = _node_packers[node.mot][type(node)]
            packer.walk(node, ctx)
            packed[node.id] = packer.pack(node)
        to_pack = [node for node in ctx.visited.values() if node.id not in packed]

    return packed[root.id], list(packed.values())


def unpack_node(root: NodeDataT, parent: Optional[NodeT]) -> NodeT:
    raise NotImplementedError


def pack_node_flat(node: NodeT) -> NodeDataT:
    """Pack a language node into a flat module node"""
    packer = _node_packers[node.mot][type(node)]
    return packer.pack(node, PackContext())


def unpack_node_flat(node: NodeDataT, parent: Optional[NodeT]) -> NodeT:
    """Unpack a flat module node into a language node"""
    packer = _node_packers[MOT_BY_DATA_CLASS[type(node)]][type(node)]
    return packer.unpack(node, parent)


@dataclass
class NodeData:
    id: UUID
    parent_id: Optional[UUID]


@dataclass
class Ordered:
    order_key: str


@dataclass
class Revisioned:
    revision: int


@dataclass(slots=True)
class ModuleData(NodeData):
    name: str
    committed: bool
    parent_id: Optional[UUID]
    nodes: Optional[list[NodeData]] = None

    def __str__(self):
        return f"{self.name}@{self.id}"

    def __repr__(self):
        return f"<Module {str(self)}>"


@node_packer(MOT.MODULE, ModuleData, language.Module)
class ModulePacker(NodePacker[ModuleData, language.Module]):
    PARENTS: ClassVar[ParentsT] = set()

    def walk(self, module: language.Module, tree: PackContext):
        for file in module.files:
            tree.visit(file)

    def pack(self, module: language.Module) -> "ModuleData":
        return ModuleData(
            id=module.id,
            name=module.name,
            committed=module.committed,
            parent_id=None,
        )

    def unpack(self, module: ModuleData, parent: Optional[language.Module]) -> language.Module:
        return language.Module(
            id=module.id,
            name=module.name,
            committed=module.committed,
            files=[],
        )

    def unwalk(self, module: language.Module, tree: UnpackContext):
        module.files = tree.get_many(module.id, language.File, recursive=True)


@dataclass(slots=True)
class FileData(NodeData, Revisioned):
    name: str

    def __str__(self):
        return f"{self.name}"

    def __repr__(self):
        return f"<File {str(self)}>"


@node_packer(MOT.FILE, FileData, language.File)
class FilePacker(NodePacker[FileData, language.File]):
    PARENTS: ClassVar[ParentsT] = {MOT.MODULE}

    def walk(self, file: language.File, tree: PackContext):
        for statement in file.statements:
            tree.visit(statement)
        for child in file.children:
            tree.visit(child)

    def pack(self, file: language.File) -> "FileData":
        return FileData(
            id=file.id,
            parent_id=file.module.id,
            name=file.name,
            revision=file.revision,
        )

    def unpack(self, file: FileData, parent: language.Module) -> language.File:
        return language.File(
            id=file.id,
            module=parent,
            name=file.name,
            revision=file.revision,
            statements=[],
            children=[],
        )

    def unwalk(self, file: language.File, tree: UnpackContext):
        file.statements = tree.get_many(file.id, language.Statement, recursive=True)
        file.children = tree.get_many(file.id, language.File)


@dataclass(slots=True)
class StatementData(NodeData, Ordered, Revisioned):
    type: StatementType
    name: Optional[str]
    text: Optional[str]
    symbol_type: Optional[StatementType]

    def __str__(self):
        parent_str = f"{self.parent_id}:" if self.parent_id else ""
        loc = str(self.parent_id) + ":" + parent_str + str(self.order_key)
        symbol_type_str = self.symbol_type.name if self.symbol_type else ""
        return f"{loc}: {self.type.name} {symbol_type_str} {self.name}"

    def __repr__(self):
        return f"<Statement {str(self)}>"


@node_packer(MOT.STATEMENT, StatementData, language.Statement)
class StatementPacker(NodePacker[StatementData, language.Statement]):
    PARENTS: ClassVar[ParentsT] = {MOT.STATEMENT, MOT.FILE}

    def walk(self, statement: language.Statement, tree: PackContext):
        for child in statement.children:
            tree.visit(child)

    def pack(self, statement: language.Statement) -> "StatementData":
        return StatementData(
            id=statement.id,
            parent_id=statement.parent.id if statement.parent else None,
            order_key=statement.order_key,
            revision=statement.revision,
            type=statement.type,
            name=statement.name,
            text=statement.text,
            symbol_type=statement.symbol_type,
        )

    def unpack(
        self, statement: StatementData, parent: language.File | language.Statement
    ) -> language.Statement:
        return language.Statement(
            id=statement.id,
            parent=parent if isinstance(parent, language.Statement) else None,
            file=parent if isinstance(parent, language.File) else parent.file,
            children=[],
            order_key=statement.order_key,
            revision=statement.revision,
            type=statement.type,
            name=statement.name,
            text=statement.text,
            symbol_type=statement.symbol_type,
        )

    def unwalk(self, statement: language.Statement, tree: UnpackContext):
        statement.children = tree.get_many(statement.id, language.Statement)


# symbols


@dataclass(slots=True)
class SymbolData(StatementData):
    symbol_type: StatementType


@dataclass(slots=True)
class TypeData(SymbolData):
    tag: Optional[TypeTag]
    flags: Optional[TypeFlag]
    description: Optional[str]


@node_packer(MOT.STATEMENT, TypeData, language.Type)
class TypePacker(StatementPacker, NodePacker[TypeData, language.Type]):
    PARENTS: ClassVar[ParentsT] = {MOT.STATEMENT}

    def walk(self, symbol: language.Type, tree: PackContext):
        super().walk(symbol, tree)
        for field in symbol.fields:
            tree.visit(field)

    def pack(self, symbol: language.Type) -> "TypeData":
        statement_data = super().pack(symbol)
        return TypeData(
            **statement_data.__dict__,
            description=symbol.description,
            tag=symbol.tag,
            flags=symbol.flags,
            symbol_type=symbol.symbol_type,
        )

    def unpack(self, symbol: TypeData, parent: language.File | language.Statement) -> language.Type:
        statement = super().unpack(symbol, parent)
        return language.Type(
            **statement.__dict__,
            description=symbol.description,
            tag=symbol.tag,
            flags=symbol.flags,
            fields=[],
        )

    def unwalk(self, symbol: language.Type, tree: UnpackContext):
        super().unwalk(symbol, tree)
        symbol.fields = tree.get_many(symbol.id, language.Field)


@dataclass(slots=True)
class TaskData(SymbolData):
    description: Optional[str]
    modifier: Optional[ExpectationModifier]


@node_packer(MOT.STATEMENT, TaskData, language.Task)
class TaskPacker(StatementPacker, NodePacker[TaskData, language.Task]):
    PARENTS: ClassVar[ParentsT] = {MOT.STATEMENT}

    def walk(self, symbol: language.Task, tree: PackContext):
        super().walk(symbol, tree)
        for field in symbol.fields:
            tree.visit(field)

    def pack(self, symbol: language.Task) -> "TaskData":
        statement_data = super().pack(symbol)
        return TaskData(
            **statement_data.__dict__,
            description=symbol.description,
            modifier=symbol.modifier,
            symbol_type=symbol.symbol_type,
        )

    def unpack(self, symbol: TaskData, parent: language.File | language.Statement) -> language.Task:
        statement = super().unpack(symbol, parent)
        return language.Task(
            **statement.__dict__,
            description=symbol.description,
            modifier=symbol.modifier,
            fields=[],
        )

    def unwalk(self, symbol: language.Task, tree: UnpackContext):
        super().unwalk(symbol, tree)
        symbol.fields = tree.get_many(symbol.id, language.Field)


@dataclass(slots=True)
class ExpectationData(SymbolData):
    description: Optional[str]
    modifier: Optional[ExpectationModifier]
    reference_id: Optional[UUID]


@node_packer(MOT.STATEMENT, ExpectationData, language.Expectation)
class ExpectationPacker(StatementPacker, NodePacker[ExpectationData, language.Expectation]):
    PARENTS: ClassVar[ParentsT] = {MOT.STATEMENT}

    def pack(self, symbol: language.Expectation) -> "ExpectationData":
        statement_data = super().pack(symbol)
        return ExpectationData(
            **statement_data.__dict__,
            modifier=symbol.modifier,
            description=symbol.description,
            symbol_type=symbol.symbol_type,
            reference_id=symbol.reference.id if symbol.reference else None,
        )

    def unpack(
        self, symbol: ExpectationData, parent: language.File | language.Statement
    ) -> language.Expectation:
        statement = super().unpack(symbol, parent)
        return language.Expectation(
            **statement.__dict__,
            modifier=symbol.modifier,
            description=symbol.description,
            reference=symbol.reference_id,
        )


@dataclass(slots=True)
class CodeData(SymbolData):
    modifier: Optional[ExpectationModifier]
    language: Optional[str]
    code: Optional[str]


@node_packer(MOT.STATEMENT, CodeData, language.Code)
class CodePacker(StatementPacker, NodePacker[CodeData, language.Code]):
    PARENTS: ClassVar[ParentsT] = {MOT.STATEMENT}

    def walk(self, symbol: language.Code, tree: PackContext):
        super().walk(symbol, tree)
        for field in symbol.fields:
            tree.visit(field)

    def pack(self, symbol: language.Code) -> "CodeData":
        statement_data = super().pack(symbol)
        return CodeData(
            **statement_data.__dict__,
            modifier=symbol.modifier,
            language=symbol.language,
            code=symbol.code,
            symbol_type=symbol.symbol_type,
        )

    def unpack(self, symbol: CodeData, parent: language.File | language.Statement) -> language.Code:
        statement = super().unpack(symbol, parent)
        return language.Code(
            **statement.__dict__,
            modifier=symbol.modifier,
            fields=[],
            language=symbol.language,
            code=symbol.code,
        )

    def unwalk(self, symbol: language.Code, tree: UnpackContext):
        super().unwalk(symbol, tree)
        symbol.fields = tree.get_many(symbol.id, language.Field)


@dataclass(slots=True)
class ModelData(SymbolData):
    external_name: Optional[str]


@node_packer(MOT.STATEMENT, ModelData, language.Model)
class ModelPacker(StatementPacker, NodePacker[ModelData, language.Model]):
    PARENTS: ClassVar[ParentsT] = {MOT.STATEMENT}

    def pack(self, symbol: language.Model) -> "ModelData":
        statement_data = super().pack(symbol)
        return ModelData(**statement_data.__dict__, external_name=symbol.external_name)

    def unpack(
        self, symbol: ModelData, parent: language.File | language.Statement
    ) -> language.Model:
        statement = super().unpack(symbol, parent)
        return language.Model(**statement.__dict__, external_name=symbol.external_name)


@dataclass(slots=True)
class RequirementData(SymbolData):
    reference_module: Optional[ModuleReference]


@node_packer(MOT.STATEMENT, RequirementData, language.Requirement)
class RequirementPacker(StatementPacker, NodePacker[RequirementData, language.Requirement]):
    PARENTS: ClassVar[ParentsT] = {MOT.STATEMENT}

    def pack(self, symbol: language.Requirement) -> "RequirementData":
        statement_data = super().pack(symbol)
        reference = (
            ModuleReference(
                module_id=symbol.module_id,
                module_name=symbol.module_name,
                version=symbol.version,
            )
            if symbol.module_id
            else None
        )
        return RequirementData(
            **statement_data.__dict__,
            reference_module=reference,
            symbol_type=symbol.symbol_type,
        )

    def unpack(
        self, symbol: RequirementData, parent: language.File | language.Statement
    ) -> language.Requirement:
        statement = super().unpack(symbol, parent)
        return language.Requirement(
            **statement.__dict__,
            module_id=symbol.reference_module.id if symbol.reference_module else None,
            module_name=symbol.reference_module.name if symbol.reference_module else None,
            version=symbol.reference_module.version if symbol.reference_module else None,
        )


@dataclass(slots=True)
class ValueData(SymbolData):
    description: Optional[str]
    tag: Optional[TypeTag]
    flags: Optional[TypeFlag]
    value: Optional[typing.Any]
    modifier: Optional[ExpectationModifier]


@node_packer(MOT.STATEMENT, ValueData, language.Value)
class ValuePacker(StatementPacker, NodePacker[ValueData, language.Value]):
    PARENTS: ClassVar[ParentsT] = {MOT.STATEMENT}

    def walk(self, symbol: language.Value, tree: PackContext):
        super().walk(symbol, tree)
        for field in symbol.fields:
            tree.visit(field)

    def pack(self, symbol: language.Value) -> "ValueData":
        statement_data = super().pack(symbol)
        return ValueData(
            **statement_data.__dict__,
            tag=symbol.tag,
            flags=symbol.flags,
            modifier=symbol.modifier,
            description=symbol.description,
            value=symbol.value,
        )

    def unpack(
        self, symbol: ValueData, parent: language.File | language.Statement
    ) -> language.Value:
        statement = super().unpack(symbol, parent)
        return language.Value(
            **statement.__dict__,
            tag=symbol.tag,
            flags=symbol.flags,
            modifier=symbol.modifier,
            description=symbol.description,
            value=symbol.value,
        )

    def unwalk(self, symbol: language.Value, tree: UnpackContext):
        super().unwalk(symbol, tree)
        symbol.fields = tree.get_many(symbol.id, language.Field)


@dataclass(slots=True)
class DatasetData(SymbolData):
    description: Optional[str]
    modifier: Optional[ExpectationModifier]
    versioned: Optional[bool]


@node_packer(MOT.STATEMENT, DatasetData, language.Dataset)
class DatasetPacker(NodePacker[DatasetData, language.Dataset]):
    PARENTS: ClassVar[ParentsT] = {MOT.STATEMENT}

    def walk(self, symbol: language.Dataset, tree: PackContext):
        super().walk(symbol, tree)
        for field in symbol.fields:
            tree.visit(field)

    def pack(self, symbol: language.Dataset) -> "DatasetData":
        statement_data = super().pack(symbol)
        return DatasetData(
            **statement_data.__dict__,
            modifier=symbol.modifier,
            description=symbol.description,
            versioned=symbol.versioned,
        )

    def unpack(self, symbol: DatasetData, parent: language.Statement) -> language.Dataset:
        statement = super().unpack(symbol, parent)
        return language.Dataset(
            **statement.__dict__,
            modifier=symbol.modifier,
            description=symbol.description,
            versioned=symbol.versioned,
            fields=[],
        )

    def unwalk(self, symbol: language.Dataset, tree: UnpackContext):
        super().unwalk(symbol, tree)
        symbol.fields = tree.get_many(symbol.id, language.Field)


SYMBOL_DATA_CLASS_BY_TYPE = {
    StatementType.TYPE: TypeData,
    StatementType.TASK: TaskData,
    StatementType.EXPECTATION: ExpectationData,
    StatementType.CODE: CodeData,
    StatementType.MODEL: ModelData,
    StatementType.REQUIREMENT: RequirementData,
    StatementType.DATASET: DatasetData,
    StatementType.VALUE: ValueData,
}
SYMBOL_TYPE_BY_DATA_CLASS = {v: k for k, v in SYMBOL_DATA_CLASS_BY_TYPE.items()}


@dataclass(slots=True)
class FieldData(NodeData, Ordered, Revisioned):
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


@node_packer(MOT.FIELD, FieldData, language.Field)
class FieldPacker(NodePacker[FieldData, language.Field]):
    PARENTS: ClassVar[ParentsT] = {MOT.STATEMENT}

    def pack(self, field: language.Field) -> "FieldData":
        return FieldData(
            id=field.id,
            order_key=field.order_key,
            revision=field.revision,
            name=field.name,
            key=field.key,
            tag=field.tag,
            hint=field.hint,
            description=field.description,
            flags=field.flags,
            reference_id=field.reference_id,
            metadata=field.metadata,
        )

    def unpack(self, field: FieldData, parent: language.Statement) -> language.Field:
        return language.Field(
            id=field.id,
            name=field.name,
            source=parent,
            revision=field.revision,
            key=field.key,
            tag=field.tag,
            hint=field.hint,
            description=field.description,
            flags=field.flags,
            reference_id=field.reference_id,
            metadata=field.metadata,
        )


@dataclass(slots=True)
class DatasetViewData(NodeData, Ordered, Revisioned):
    PARENTS: ClassVar[ParentsT] = {MOT.STATEMENT}

    id: UUID
    name: str
    query: Optional[Query] = None
    sort: Optional[list[Sort]] = None
    length: Optional[int] = None
    reference_id: Optional[UUID] = None


@node_packer(MOT.STATEMENT, DatasetViewData, language.DatasetView)
class DatasetViewPacker(NodePacker[DatasetViewData, language.DatasetView]):
    PARENTS: ClassVar[ParentsT] = {MOT.STATEMENT}

    def pack(self, view: language.DatasetView) -> "DatasetViewData":
        return DatasetViewData(
            id=view.id,
            order_key=view.order_key,
            name=view.name,
            query=view.query,
            sort=view.sort,
            reference_id=view.reference.id if view.reference else None,
        )

    def unpack(self, view: DatasetViewData, parent: language.Statement) -> language.DatasetView:
        return language.DatasetView(
            id=view.id,
            name=view.name,
            source=parent,
            query=view.query,
            sort=view.sort,
            reference=view.reference_id,
        )


@dataclass(slots=True)
class RecordData(NodeData, Ordered, Revisioned):
    PARENTS: ClassVar[ParentsT] = {MOT.STATEMENT}

    data: Optional[typing.Any] = None

    def __str__(self):
        return f"{self.parent_id}:{self.order_key} {describe_type(self.data)}"

    def __repr__(self):
        return f"<{self.__class__.__name__} {str(self)}>"


@node_packer(MOT.STATEMENT, RecordData, language.Record)
class RecordPacker(NodePacker[RecordData, language.Record]):
    PARENTS: ClassVar[ParentsT] = {MOT.STATEMENT}

    def pack(self, record: language.Record) -> "RecordData":
        return RecordData(
            id=record.id,
            order_key=record.order_key,
            revision=record.revision,
            data=record.data,
        )

    def unpack(self, record: "RecordData", parent: language.Statement) -> language.Record:
        return language.Record(
            id=record.id,
            revision=record.revision,
            data=record.data,
        )


# interp


@dataclass(slots=True)
class InterpData(NodeData):
    scope: InterpScope
    issues: Optional[list["IssueData"]] = None
    resolved_fields: Optional[list[FieldData]] = None

    def hash_content(self) -> str:
        content = (
            self.parent_id,
            *(issue.id for issue in (self.issues or [])),
            *(field.id for field in (self.resolved_fields or [])),
        )
        content = str(content).encode("utf-8")
        return md5(content).hexdigest()


@node_packer(MOT.INTERP, InterpData, None)
class InterpPacker(NodePacker[InterpData, None]):
    PARENTS: ClassVar[ParentsT] = {MOT.MODULE, MOT.STATEMENT, MOT.FILE}


@dataclass(slots=True)
class IssueData(NodeData):
    scope: InterpScope
    kind: IssueKind
    type: IssueType
    message: Optional[str]


# other objects


_data_packers: dict[typing.Type[DataT], "DataPacker"] = {}


def data_packer(data_t: typing.Type[DataT]):
    """Decorator to register a data packer for a given type"""

    def decorator(cls: "DataPacker"):
        if data_t in _data_packers:
            raise ValueError(f"packer for {data_t} already registered: {_data_packers[data_t]}")
        _data_packers[data_t] = cls
        return cls

    return decorator


def pack_data(data: ObjectT) -> DataT:
    """Pack a language data object into a flat module node"""
    packer = _data_packers[type(data)]
    return packer.pack(data)


def unpack_data(data: DataT) -> ObjectT:
    """Unpack a flat module node into a language data object"""
    packer = _data_packers[type(data)]
    return packer.unpack(data)


@dataclass(slots=True)
class XBlockData:
    kind: str
    source: str
    value: Optional[typing.Any] = None
    path: Optional[str] = None

    def __str__(self):
        return f"{self.kind} {self.source}"

    def __repr__(self):
        return f"<XBlock {str(self)}>"


@data_packer(XBlockData)
class XBlockPacker(DataPacker[XBlockData, "XBlock"]):
    def pack(self, object: "XBlock") -> XBlockData:
        return XBlockData(
            kind=object.kind, source=object.source, value=object.value, path=object.path
        )

    def unpack(self, data: XBlockData) -> "XBlock":
        from bench.bench.build import XBlock

        return XBlock(kind=data.kind, source=data.source, value=data.value, path=data.path)


@dataclass(slots=True)
class RemoteObjectData:
    id: UUID
    sha512: str
    content_length: int
    content_type: str
    name: Optional[str]

    def __str__(self):
        return f"{self.id} {self.name} ({self.content_type}, {self.content_length} bytes)"

    def __repr__(self):
        return f"<RemoteObject {self}>"


@data_packer(RemoteObjectData)
class RemoteObjectPacker(DataPacker[RemoteObjectData, language.RemoteObject]):
    def pack(self, object: language.RemoteObject) -> RemoteObjectData:
        return RemoteObjectData(
            id=object.id,
            sha512=object.sha512,
            content_length=object.content_length,
            content_type=object.content_type,
            name=object.name,
        )

    def unpack(self, data: RemoteObjectData) -> language.RemoteObject:
        return language.RemoteObject(
            id=data.id,
            sha512=data.sha512,
            content_length=data.content_length,
            content_type=data.content_type,
            name=data.name,
        )


@dataclass(slots=True)
class SecretData:
    id: UUID
    sha512: str
    value: Optional[typing.Any] = None

    def __str__(self):
        return f"{self.id} ({self.sha512})"

    def __repr__(self):
        return f"<Secret {self}>"


@data_packer(SecretData)
class SecretPacker(DataPacker[SecretData, language.Secret]):
    def pack(self, object: language.Secret) -> SecretData:
        return SecretData(id=object.id, sha512=object.sha512, value=object.value)

    def unpack(self, data: SecretData) -> language.Secret:
        return language.Secret(id=data.id, sha512=data.sha512, value=data.value)


@dataclass(slots=True)
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
    error: Optional[RunErrorData]
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
            if isinstance(frame.runnable, Code):
                stack = PyFrameData.from_stack(stack_summary)
                stack = PyFrameData.clean(stack, frame.runnable, session=session)
            else:
                stack = []
            error_str = str(frame.error)
            # remove (source=...) from error message
            error_str = re.sub(r"\(source=.+\)", "", error_str)
            error_data = RunErrorData(
                type=type(frame.error).__name__,
                symbol=str(frame.runnable),
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
