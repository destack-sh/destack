import abc
import enum
import typing
from collections import defaultdict
from dataclasses import dataclass, fields
from hashlib import md5
from typing import ClassVar, Optional
from uuid import UUID

from bench import language
from bench.language import IssueType, StatementType, SymbolType, TypeHint, TypeTag
from bench.language.const import ExpectationModifier, InterpScope, TypeFlag
from bench.language.dataset import Query, Sort
from bench.language.issue import IssueKind
from bench.language.type import SYMBOL_CLASS_BY_TYPE, LanguageObject, ModuleReference
from bench.utils.func import describe_type

if typing.TYPE_CHECKING:
    from bench.language.build import XBlock


#
# Stable, concise and flat language data structures for transit and storage.
# TODO @Performance @Robustness: use an optimized and evolvable :WireFormat
#


class ModuleObjectType(enum.StrEnum):
    MODULE = "MODULE"
    FILE = "FILE"
    STATEMENT = "STATEMENT"
    SYMBOL = "SYMBOL"
    FIELD = "FIELD"
    RECORD = "RECORD"
    DATASET_VIEW = "DATASET_VIEW"
    INTERP = "INTERP"


MOT = ModuleObjectType
ParentsT = set[MOT]
NodeDataT = typing.TypeVar("NodeDataT", bound="NodeData")
NodeT = typing.TypeVar("NodeT", bound="LanguageObject")
DataT = typing.TypeVar("DataT")
ObjectT = typing.TypeVar("ObjectT")


class DataPacker(typing.Generic[DataT, ObjectT]):
    """Generic data packer for non-node data types"""

    def pack(self, object: ObjectT) -> DataT:
        raise NotImplementedError

    def unpack(self, data: DataT) -> ObjectT:
        raise NotImplementedError


class TreeVis(abc.ABC):
    """Tree visitor for packing/unpacking"""

    def visit(self, node: LanguageObject):
        raise NotImplementedError

    def get_one(
        self, parent_id: UUID, t: typing.Type[LanguageObject]
    ) -> Optional["LanguageObject"]:
        raise NotImplementedError

    def get_many(
        self, parent_id: UUID, t: typing.Type[LanguageObject], recursive: bool = False
    ) -> list["LanguageObject"]:
        raise NotImplementedError


class NodePacker(abc.ABC, typing.Generic[NodeDataT, NodeT]):
    """Module node packer"""

    def pack(self, node: NodeT, tree: TreeVis) -> NodeDataT:
        raise NotImplementedError

    def unpack(self, node: NodeDataT, parent: Optional[NodeT], tree: TreeVis) -> NodeT:
        raise NotImplementedError


# registered packers, where each MOT may have multiple packers (subtypes)
_node_packers: dict[MOT, dict[typing.Type[NodeDataT], "NodePacker"]] = defaultdict(dict)
_data_packers: dict[typing.Type[DataT], "DataPacker"] = {}


def node_packer(t: MOT, node_t: typing.Type[NodeDataT]):
    """Decorator to register a node packer for a given type"""

    def decorator(cls: "NodePacker"):
        if node_t in _node_packers[t]:
            raise ValueError(
                f"packer for {t} and {node_t} already registered: {_node_packers[t][node_t]}"
            )
        _node_packers[t][node_t] = cls
        return cls

    return decorator


def data_packer(data_t: typing.Type[DataT]):
    """Decorator to register a data packer for a given type"""

    def decorator(cls: "DataPacker"):
        if data_t in _data_packers:
            raise ValueError(f"packer for {data_t} already registered: {_data_packers[data_t]}")
        _data_packers[data_t] = cls
        return cls

    return decorator


def pack_node_flat(node: NodeT, tree: "TreeVis") -> NodeDataT:
    """Pack a language node into a flat module node"""
    packer = _node_packers[node.mot][type(node)]
    return packer.pack(node, tree)


def unpack_node_flat(node: NodeDataT, parent: Optional[NodeT], tree: "TreeVis") -> NodeT:
    """Unpack a flat module node into a language node"""
    packer = _node_packers[node.mot][type(node)]
    return packer.unpack(node, parent, tree)


def pack_data(data: ObjectT) -> DataT:
    """Pack a language data object into a flat module node"""
    packer = _data_packers[type(data)]
    return packer.pack(data)


def unpack_data(data: DataT) -> ObjectT:
    """Unpack a flat module node into a language data object"""
    packer = _data_packers[type(data)]
    return packer.unpack(data)


@dataclass(slots=True)
class NodeData:
    id: UUID
    parent_id: Optional[UUID]


@dataclass(slots=True)
class Ordered:
    order_key: str


@dataclass(slots=True)
class ModuleData:
    id: UUID
    name: str
    committed: bool

    def __str__(self):
        return f"{self.name}@{self.id} ({len(self.nodes)} nodes)"

    def __repr__(self):
        return f"<Module {str(self)}>"


@node_packer(MOT.MODULE, ModuleData)
class ModulePacker(NodePacker[ModuleData, language.Module]):
    PARENTS: ClassVar[ParentsT] = set()

    def pack(self, module: language.Module, tree: TreeVis) -> "ModuleData":
        for file in module.files:
            tree.visit(file)
        return ModuleData(
            id=module.id,
            name=module.name,
            committed=module.committed,
        )

    def unpack(
        self, module: ModuleData, parent: Optional[language.Module], tree: TreeVis
    ) -> language.Module:
        files = tree.get_many(module.id, language.File, recursive=True)
        return language.Module(
            id=module.id,
            name=module.name,
            committed=module.committed,
            files=files,
        )


@dataclass(slots=True)
class FileData(NodeData):
    name: str
    revision: int

    def __str__(self):
        return f"{self.name}"

    def __repr__(self):
        return f"<File {str(self)}>"


@node_packer(MOT.FILE, FileData)
class FilePacker(NodePacker[FileData, language.File]):
    PARENTS: ClassVar[ParentsT] = {MOT.MODULE}

    def pack(self, file: language.File, tree: TreeVis) -> "FileData":
        for statement in file.statements:
            tree.visit(statement)
        for child in file.children:
            tree.visit(child)
        return FileData(
            id=file.id,
            parent_id=file.module.id,
            name=file.name,
            revision=file.revision,
        )

    def unpack(self, file: FileData, parent: language.Module, tree: TreeVis) -> language.File:
        statements = tree.get_many(file.id, language.Statement, recursive=True)
        files = tree.get_many(file.id, language.File)
        return language.File(
            id=file.id,
            module=parent,
            name=file.name,
            revision=file.revision,
            statements=statements,
            children=files,
        )


@dataclass(slots=True)
class StatementData(NodeData, Ordered):
    revision: int
    type: StatementType
    name: Optional[str]
    fqn: Optional[str]
    text: Optional[str]
    symbol_type: Optional[SymbolType]

    def __str__(self):
        parent_str = f"{self.parent_id}:" if self.parent_id else ""
        loc = str(self.parent_id) + ":" + parent_str + str(self.order_key)
        symbol_type_str = self.symbol_type.name if self.symbol_type else ""
        return f"{loc}: {self.type.name} {symbol_type_str} {self.name}"

    def __repr__(self):
        return f"<Statement {str(self)}>"


@node_packer(MOT.STATEMENT, StatementData)
class StatementPacker:
    PARENTS: ClassVar[ParentsT] = {MOT.STATEMENT, MOT.FILE}

    def pack(self, statement: language.Statement, tree: TreeVis) -> "StatementData":
        if statement.symbol is not None:
            tree.visit(statement.symbol)
        for child in statement.children:
            tree.visit(child)
        return StatementData(
            id=statement.id,
            parent_id=statement.parent.id if statement.parent else None,
            order_key=statement.order_key,
            revision=statement.revision,
            type=statement.type,
            name=statement.name,
            fqn=statement.fqn,
            text=statement.text,
            symbol_type=statement.symbol_type,
        )

    def unpack(
        self, statement: StatementData, parent: language.File | language.Statement, tree: TreeVis
    ) -> language.Statement:
        symbol = tree.get_one(statement.id, language.Symbol)
        children = tree.get_many(statement.id, language.Statement)
        return language.Statement(
            id=statement.id,
            parent=parent if isinstance(parent, language.Statement) else None,
            file=parent if isinstance(parent, language.File) else parent.file,
            children=children,
            order_key=statement.order_key,
            revision=statement.revision,
            type=statement.type,
            name=statement.name,
            fqn=statement.fqn,
            text=statement.text,
            symbol_type=statement.symbol_type,
            symbol=symbol,
        )


_STATEMENT_DATA_FIELDS = fields(StatementData)


# symbols


@dataclass(slots=True)
class SymbolData(NodeData):
    type: SymbolType


@node_packer(MOT.SYMBOL, SymbolData)
class SymbolPacker(NodePacker[SymbolData, language.Symbol]):
    PARENTS: ClassVar[ParentsT] = {MOT.STATEMENT}

    def pack(self, symbol: language.Symbol, tree: TreeVis) -> "SymbolData":
        cls = SYMBOL_DATA_CLASS_BY_TYPE[symbol.symbol_type]
        return cls(id=symbol.id, parent_id=symbol.id)

    def unpack(
        self, symbol: SymbolData, parent: language.Statement, tree: TreeVis
    ) -> language.Symbol:
        cls = SYMBOL_CLASS_BY_TYPE[symbol.type]
        return cls(id=symbol.id, parent=parent)


@dataclass(slots=True)
class TypeData(SymbolData):
    tag: Optional[TypeTag]
    flags: Optional[TypeFlag]
    description: Optional[str]


@node_packer(MOT.SYMBOL, TypeData)
class TypePacker(NodePacker[TypeData, language.Type]):
    PARENTS: ClassVar[ParentsT] = {MOT.STATEMENT}

    def pack(self, symbol: language.Type, tree: TreeVis) -> "TypeData":
        for field in symbol.fields:
            tree.visit(field)
        return TypeData(
            id=symbol.id,
            parent_id=symbol.id,
            description=symbol.description,
            tag=symbol.tag,
            flags=symbol.flags,
        )

    def unpack(self, symbol: TypeData, parent: language.Statement, tree: TreeVis) -> language.Type:
        fields = tree.get_many(symbol.id, language.Field)
        return language.Type(
            id=symbol.id,
            name=parent.name,
            source=parent,
            description=symbol.description,
            tag=symbol.tag,
            flags=symbol.flags,
            fields=fields,
        )


@dataclass(slots=True)
class TaskData(SymbolData):
    description: Optional[str]
    modifier: Optional[ExpectationModifier]


@node_packer(MOT.SYMBOL, TaskData)
class TaskPacker(NodePacker[TaskData, language.Task]):
    PARENTS: ClassVar[ParentsT] = {MOT.STATEMENT}

    def pack(self, symbol: language.Task, tree: TreeVis) -> "TaskData":
        for field in symbol.fields:
            tree.visit(field)
        return TaskData(
            id=symbol.id,
            parent_id=symbol.id,
            modifier=symbol.modifier,
            description=symbol.description,
        )

    def unpack(self, symbol: TaskData, parent: language.Statement, tree: TreeVis) -> language.Task:
        fields = tree.get_many(symbol.id, language.Field)
        return language.Task(
            id=symbol.id,
            name=parent.name,
            source=parent,
            modifier=symbol.modifier,
            description=symbol.description,
            fields=fields,
        )


@dataclass(slots=True)
class ExpectationData(SymbolData):
    description: Optional[str]
    modifier: Optional[ExpectationModifier]


@node_packer(MOT.SYMBOL, ExpectationData)
class ExpectationPacker(NodePacker[ExpectationData, language.Expectation]):
    PARENTS: ClassVar[ParentsT] = {MOT.STATEMENT}

    def pack(self, symbol: language.Expectation, tree: TreeVis) -> "ExpectationData":
        return ExpectationData(
            id=symbol.id,
            parent_id=symbol.id,
            modifier=symbol.modifier,
            description=symbol.description,
        )

    def unpack(
        self, symbol: ExpectationData, parent: language.Statement, tree: TreeVis
    ) -> language.Expectation:
        return language.Expectation(
            id=symbol.id,
            name=parent.name,
            source=parent,
            modifier=symbol.modifier,
            description=symbol.description,
            fields=fields,
        )


@dataclass(slots=True)
class CodeData(SymbolData):
    modifier: Optional[ExpectationModifier]
    lang: Optional[str]
    code: Optional[str]


@node_packer(MOT.SYMBOL, CodeData)
class CodePacker(NodePacker[CodeData, language.Code]):
    PARENTS: ClassVar[ParentsT] = {MOT.STATEMENT}

    def pack(self, symbol: language.Code, tree: TreeVis) -> "CodeData":
        for field in symbol.fields:
            tree.visit(field)
        return CodeData(
            id=symbol.id,
            parent_id=symbol.id,
            modifier=symbol.modifier,
            description=symbol.description,
            lang=symbol.lang,
            code=symbol.code,
        )

    def unpack(self, symbol: CodeData, parent: language.Statement, tree: TreeVis) -> language.Code:
        fields = tree.get_many(symbol.id, language.Field)
        return language.Code(
            id=symbol.id,
            name=parent.name,
            source=parent,
            modifier=symbol.modifier,
            description=symbol.description,
            fields=fields,
            lang=symbol.lang,
            code=symbol.code,
        )


@dataclass(slots=True)
class ModelData(SymbolData):
    external_name: Optional[str]


@node_packer(MOT.SYMBOL, ModelData)
class ModelPacker(NodePacker[ModelData, language.Model]):
    PARENTS: ClassVar[ParentsT] = {MOT.STATEMENT}

    def pack(self, symbol: language.Model, tree: TreeVis) -> "ModelData":
        return ModelData(
            id=symbol.id,
            parent_id=symbol.id,
            modifier=symbol.modifier,
            description=symbol.description,
            external_name=symbol.external_name,
        )

    def unpack(
        self, symbol: ModelData, parent: language.Statement, tree: TreeVis
    ) -> language.Model:
        return language.Model(
            id=symbol.id,
            name=parent.name,
            source=parent,
            modifier=symbol.modifier,
            description=symbol.description,
            external_name=symbol.external_name,
        )


@dataclass(slots=True)
class RequirementData(SymbolData):
    reference_module: Optional[ModuleReference]


@node_packer(MOT.SYMBOL, RequirementData)
class RequirementPacker(NodePacker[RequirementData, language.Requirement]):
    PARENTS: ClassVar[ParentsT] = {MOT.STATEMENT}

    def pack(self, symbol: language.Requirement, tree: TreeVis) -> "RequirementData":
        return RequirementData(
            id=symbol.id,
            parent_id=symbol.id,
            reference_module=ModuleReference(
                module_id=symbol.module_id,
                module_name=symbol.module_name,
                version=symbol.version,
            )
            if symbol.module_id
            else None,
        )

    def unpack(
        self, symbol: RequirementData, parent: language.Statement, tree: TreeVis
    ) -> language.Requirement:
        return language.Requirement(
            id=symbol.id,
            name=parent.name,
            source=parent,
            module_id=symbol.reference_module.id,
            module_name=symbol.reference_module.name,
            version=symbol.reference_module.version,
        )


@dataclass(slots=True)
class ValueData(SymbolData):
    description: Optional[str]
    value: Optional[typing.Any]
    modifier: Optional[ExpectationModifier]


@node_packer(MOT.SYMBOL, ValueData)
class ValuePacker(NodePacker[ValueData, language.Value]):
    PARENTS: ClassVar[ParentsT] = {MOT.STATEMENT}

    def pack(self, symbol: language.Value, tree: TreeVis) -> "ValueData":
        for field in symbol.fields:
            tree.visit(field)
        return ValueData(
            id=symbol.id,
            parent_id=symbol.id,
            modifier=symbol.modifier,
            description=symbol.description,
            value=symbol.value,
        )

    def unpack(
        self, symbol: ValueData, parent: language.Statement, tree: TreeVis
    ) -> language.Value:
        fields = tree.get_many(symbol.id, language.Field)
        return language.Value(
            id=symbol.id,
            name=parent.name,
            source=parent,
            modifier=symbol.modifier,
            description=symbol.description,
            fields=fields,
            value=symbol.value,
        )


@dataclass(slots=True)
class DatasetData(SymbolData):
    description: Optional[str]
    modifier: Optional[ExpectationModifier]


@node_packer(MOT.SYMBOL, DatasetData)
class DatasetPacker(NodePacker[DatasetData, language.Dataset]):
    PARENTS: ClassVar[ParentsT] = {MOT.STATEMENT}

    def pack(self, symbol: language.Dataset, tree: TreeVis) -> "DatasetData":
        for field in symbol.fields:
            tree.visit(field)
        return DatasetData(
            id=symbol.id,
            parent_id=symbol.id,
            modifier=symbol.modifier,
            description=symbol.description,
        )

    def unpack(
        self, symbol: DatasetData, parent: language.Statement, tree: TreeVis
    ) -> language.Dataset:
        fields = tree.get_many(symbol.id, language.Field)
        return language.Dataset(
            id=symbol.id,
            name=parent.name,
            source=parent,
            modifier=symbol.modifier,
            description=symbol.description,
            fields=fields,
        )


SYMBOL_DATA_CLASS_BY_TYPE = {
    SymbolType.TYPE: TypeData,
    SymbolType.TASK: TaskData,
    SymbolType.EXPECTATION: ExpectationData,
    SymbolType.CODE: CodeData,
    SymbolType.MODEL: ModelData,
    SymbolType.REQUIREMENT: RequirementData,
    SymbolType.DATASET: DatasetData,
    SymbolType.VALUE: ValueData,
}
SYMBOL_TYPE_BY_DATA_CLASS = {v: k for k, v in SYMBOL_DATA_CLASS_BY_TYPE.items()}


@dataclass(slots=True)
class FieldData(NodeData, Ordered):
    revision: int
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


@node_packer(MOT.FIELD, FieldData)
class FieldPacker(NodePacker[FieldData, language.Field]):
    PARENTS: ClassVar[ParentsT] = {MOT.FIELD}

    def pack(self, field: language.Field, tree: TreeVis) -> "FieldData":
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

    def unpack(self, field: FieldData, parent: language.Statement, tree: TreeVis) -> language.Field:
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
class DatasetViewData(NodeData, Ordered):
    PARENTS: ClassVar[ParentsT] = {MOT.SYMBOL}

    id: UUID
    name: str
    query: Optional[Query] = None
    sort: Optional[list[Sort]] = None
    length: Optional[int] = None
    reference_id: Optional[UUID] = None


@node_packer(MOT.SYMBOL, DatasetViewData)
class DatasetViewPacker(NodePacker[DatasetViewData, language.DatasetView]):
    PARENTS: ClassVar[ParentsT] = {MOT.SYMBOL}

    def pack(self, view: language.DatasetView, tree: TreeVis) -> "DatasetViewData":
        return DatasetViewData(
            id=view.id,
            order_key=view.order_key,
            name=view.name,
            query=view.query,
            sort=view.sort,
            reference_id=view.reference.id if view.reference else None,
        )

    def unpack(
        self, view: DatasetViewData, parent: language.Statement, tree: TreeVis
    ) -> language.DatasetView:
        return language.DatasetView(
            id=view.id,
            name=view.name,
            source=parent,
            query=view.query,
            sort=view.sort,
            reference=view.reference_id,
        )


@dataclass(slots=True)
class RecordData(NodeData, Ordered):
    PARENTS = {MOT.SYMBOL}

    revision: int
    data: Optional[typing.Any] = None

    def __str__(self):
        return f"{self.parent_id}:{self.order_key} {describe_type(self.data)}"

    def __repr__(self):
        return f"<{self.__class__.__name__} {str(self)}>"


@node_packer(MOT.SYMBOL, RecordData)
class RecordPacker(NodePacker[RecordData, language.Record]):
    PARENTS: ClassVar[ParentsT] = {MOT.SYMBOL}

    def pack(self, record: language.Record, tree: TreeVis) -> "RecordData":
        return RecordData(
            id=record.id,
            order_key=record.order_key,
            revision=record.revision,
            data=record.data,
        )

    def unpack(
        self, record: "RecordData", parent: language.Statement, tree: TreeVis
    ) -> language.Record:
        return language.Record(
            id=record.id,
            name=parent.name,
            source=parent,
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


@node_packer(MOT.INTERP, InterpData)
class InterpPacker(NodePacker[InterpData, None]):
    PARENTS: ClassVar[ParentsT] = {MOT.MODULE, MOT.STATEMENT, MOT.FILE}


@dataclass(slots=True)
class IssueData(NodeData):
    scope: InterpScope
    kind: IssueKind
    type: IssueType
    message: Optional[str]


# other objects


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


@data_packer(XBlockData, "XBlock")
class XBlockPacker(DataPacker[XBlockData, "XBlock"]):
    def pack(self, object: "XBlock") -> XBlockData:
        return XBlockData(
            kind=object.kind,
            source=object.source,
            value=object.value,
            path=object.path,
        )

    def unpack(self, data: XBlockData) -> "XBlock":
        from bench.language.build import XBlock

        return XBlock(
            kind=data.kind,
            source=data.source,
            value=data.value,
            path=data.path,
        )


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


@data_packer(RemoteObjectData, language.RemoteObject)
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


@data_packer(SecretData, language.Secret)
class SecretPacker(DataPacker[SecretData, language.Secret]):
    def pack(self, object: language.Secret) -> SecretData:
        return SecretData(
            id=object.id,
            sha512=object.sha512,
            value=object.value,
        )

    def unpack(self, data: SecretData) -> language.Secret:
        return language.Secret(
            id=data.id,
            sha512=data.sha512,
            value=data.value,
        )
