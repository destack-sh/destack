import abc
import enum
import typing
from dataclasses import asdict, dataclass, fields
from hashlib import md5
from typing import ClassVar, Optional
from uuid import UUID

from bench import language
from bench.language import (
    IssueType,
    StatementModifier,
    StatementType,
    SymbolType,
    TypeHint,
    TypeTag,
)
from bench.language.const import InterpScope, TypeFlag
from bench.language.dataset import Query, Sort
from bench.language.issue import IssueKind
from bench.language.type import SYMBOL_CLASS_BY_TYPE, LanguageObject, ModuleReference
from bench.utils.func import describe_type

if typing.TYPE_CHECKING:
    pass

#
# Stable, concise and flat language data structures for transit and storage.
# TODO @Performance: use an optimized and evolvable wire format :WireFormat
#

Parents = set["ModuleObjectType"]


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
ModuleNodeT = typing.TypeVar("ModuleNodeT", bound="ModuleNode")
LanguageNodeT = typing.TypeVar("LanguageNodeT", bound="LanguageObject")


class TreeP(abc.ABC):
    def visit(self, node: LanguageObject):
        raise NotImplementedError

    def get_one(
        self, parent_id: UUID, t: typing.Type[LanguageObject]
    ) -> Optional["LanguageObject"]:
        raise NotImplementedError

    def get_many(self, parent_id: UUID, t: typing.Type[LanguageObject]) -> list["LanguageObject"]:
        raise NotImplementedError


@dataclass(slots=True)
class ModuleNode:
    id: UUID
    parent_id: Optional[UUID]


@dataclass(slots=True)
class Ordered:
    order_key: str


@dataclass(slots=True)
class ModuleData:
    id: UUID
    name: str
    nodes: list[ModuleNode]
    committed: bool

    def __str__(self):
        return f"{self.name}@{self.id} ({len(self.files)} files)"

    def __repr__(self):
        return f"<Module {str(self)}>"


@dataclass(slots=True)
class FileData(ModuleNode):
    PARENTS: ClassVar[Parents] = {MOT.MODULE}

    path: str
    revision: int

    def __str__(self):
        return f"{self.path}"

    @property
    def name(self):
        # name property to emulate File model
        return self.path.split(".")[-1]

    def __repr__(self):
        return f"<File {str(self)}>"

    @staticmethod
    def pack(file: language.File, tree: TreeP) -> "FileData":
        for statement in file.statements:
            tree.visit(statement)
        return FileData(
            id=file.id,
            parent_id=file.module.id,
            path=file.path,
            revision=file.revision,
        )

    def unpack(self, parent: language.Module) -> language.File:
        return language.File(
            id=self.id,
            module=parent,
            path=self.path,
            revision=self.revision,
        )


@dataclass(slots=True)
class StatementData(ModuleNode, Ordered):
    PARENTS: ClassVar[Parents] = {MOT.STATEMENT, MOT.FILE}

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

    @staticmethod
    def pack(statement: language.Statement, tree: TreeP) -> "StatementData":
        if statement.symbol is not None:
            tree.visit(statement.symbol)
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

    def unpack(self, parent: language.File | language.Statement, tree: TreeP) -> language.Statement:
        symbol = tree.get_one(self.id, language.Symbol)
        return language.Statement(
            id=self.id,
            parent=parent if isinstance(parent, language.Statement) else None,
            file=parent if isinstance(parent, language.File) else parent.file,
            order_key=self.order_key,
            revision=self.revision,
            type=self.type,
            name=self.name,
            fqn=self.fqn,
            text=self.text,
            symbol_type=self.symbol_type,
            symbol=symbol,
        )


_STATEMENT_DATA_FIELDS = fields(StatementData)


# symbols


@dataclass(slots=True)
class SymbolData(ModuleNode):
    PARENTS: ClassVar[Parents] = {MOT.STATEMENT}

    type: SymbolType
    modifier: Optional[StatementModifier]  # should really be in symbol data...
    description: Optional[str]

    @staticmethod
    def pack(symbol: language.Symbol, tree: TreeP) -> "SymbolData":
        cls = SYMBOL_DATA_CLASS_BY_TYPE[symbol.symbol_type]
        return cls(
            id=symbol.id,
            parent_id=symbol.id,
            modifier=symbol.modifier,
            description=symbol.description,
        )

    def unpack(self, parent: language.Statement, tree: TreeP) -> language.Symbol:
        cls = SYMBOL_CLASS_BY_TYPE[self.type]
        return cls(
            id=self.id,
            parent=parent,
            modifier=self.modifier,
            description=self.description,
        )


@dataclass(slots=True)
class TypeData(SymbolData):
    tag: Optional[TypeTag]
    flags: Optional[TypeFlag]

    @staticmethod
    def pack(symbol: language.Type, tree: TreeP) -> "TypeData":
        return TypeData(
            id=symbol.id,
            parent_id=symbol.id,
            modifier=symbol.modifier,
            description=symbol.description,
            tag=symbol.tag,
            flags=symbol.flags,
        )

    def unpack(self, parent: language.Statement, tree: TreeP) -> language.Type:
        return language.Type(
            id=self.id,
            name=parent.name,
            source=parent,
            modifier=self.modifier,
            description=self.description,
            tag=self.tag,
            flags=self.flags,
        )


@dataclass(slots=True)
class TaskData(SymbolData):
    @staticmethod
    def pack(symbol: language.Task, tree: TreeP) -> "TaskData":
        for field in symbol.fields:
            tree.visit(field)
        return TaskData(
            id=symbol.id,
            parent_id=symbol.id,
            modifier=symbol.modifier,
            description=symbol.description,
        )

    def unpack(self, parent: language.Statement, tree: TreeP) -> language.Task:
        fields = tree.get_many(self.id, language.Field)
        return language.Task(
            id=self.id,
            name=parent.name,
            source=parent,
            modifier=self.modifier,
            description=self.description,
            fields=fields,
        )


@dataclass(slots=True)
class ExpectationData(SymbolData):
    pass


@dataclass(slots=True)
class CodeData(SymbolData):
    lang: Optional[str]
    code: Optional[str]

    @staticmethod
    def pack(symbol: language.Code, tree: TreeP) -> "CodeData":
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

    def unpack(self, parent: language.Statement, tree: TreeP) -> language.Code:
        fields = tree.get_many(self.id, language.Field)
        return language.Code(
            id=self.id,
            name=parent.name,
            source=parent,
            modifier=self.modifier,
            description=self.description,
            fields=fields,
            lang=self.lang,
            code=self.code,
        )


@dataclass(slots=True)
class ModelData(SymbolData):
    external_name: Optional[str]

    @staticmethod
    def pack(symbol: language.Model, tree: TreeP) -> "ModelData":
        return ModelData(
            id=symbol.id,
            parent_id=symbol.id,
            modifier=symbol.modifier,
            description=symbol.description,
            external_name=symbol.external_name,
        )

    def unpack(self, parent: language.Statement, tree: TreeP) -> language.Model:
        return language.Model(
            id=self.id,
            name=parent.name,
            source=parent,
            modifier=self.modifier,
            description=self.description,
            external_name=self.external_name,
        )


@dataclass(slots=True)
class RequirementData(SymbolData):
    reference_module: Optional[ModuleReference]

    @staticmethod
    def pack(symbol: language.Requirement, tree: TreeP) -> "RequirementData":
        return RequirementData(
            id=symbol.id,
            parent_id=symbol.id,
            modifier=symbol.modifier,
            description=symbol.description,
            reference_module=ModuleReference(
                module_id=symbol.module_id,
                module_name=symbol.module_name,
                version=symbol.version,
            )
            if symbol.module_id
            else None,
        )

    def unpack(self, parent: language.Statement, tree: TreeP) -> language.Requirement:
        return language.Requirement(
            id=self.id,
            name=parent.name,
            source=parent,
            modifier=self.modifier,
            description=self.description,
            module_id=self.reference_module.id,
            module_name=self.reference_module.name,
            version=self.reference_module.version,
        )


@dataclass(slots=True)
class ValueData(SymbolData):
    value: Optional[typing.Any]

    @staticmethod
    def pack(symbol: language.Value, tree: TreeP) -> "ValueData":
        for field in symbol.fields:
            tree.visit(field)
        return ValueData(
            id=symbol.id,
            parent_id=symbol.id,
            modifier=symbol.modifier,
            description=symbol.description,
            value=symbol.value,
        )


@dataclass(slots=True)
class DatasetData(SymbolData):
    @staticmethod
    def pack(symbol: language.Dataset, tree: TreeP) -> "DatasetData":
        for field in symbol.fields:
            tree.visit(field)
        return DatasetData(
            id=symbol.id,
            parent_id=symbol.id,
            modifier=symbol.modifier,
            description=symbol.description,
        )

    def unpack(self, parent: language.Statement, tree: TreeP) -> language.Dataset:
        fields = tree.get_many(self.id, language.Field)
        return language.Dataset(
            id=self.id,
            name=parent.name,
            source=parent,
            modifier=self.modifier,
            description=self.description,
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


@dataclass(slots=True)
class FieldData(ModuleNode, Ordered):
    PARENTS: ClassVar[Parents] = {MOT.SYMBOL}

    revision: int
    name: Optional[str]
    key: str
    tag: TypeTag
    hint: Optional[TypeHint]
    description: Optional[str]
    flags: TypeFlag
    value: Optional[typing.Any] = None
    reference_id: Optional[UUID] = None
    metadata: Optional[typing.Any] = None

    def __str__(self):
        name_str = f"{self.name} " if self.name else ""
        return f"{self.parent_id}:{self.order_key} {name_str}{self.tag.value}"

    def __repr__(self):
        return f"<Field {str(self)}>"

    @staticmethod
    def pack(field: language.Field) -> "FieldData":
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
            value=field.value,
            reference_id=field.reference_id,
            metadata=field.metadata,
        )


@dataclass(slots=True)
class DatasetViewData(ModuleNode, Ordered):
    PARENTS: ClassVar[Parents] = {MOT.SYMBOL}

    id: UUID
    name: str
    query: Optional[Query] = None
    sort: Optional[list[Sort]] = None
    length: Optional[int] = None
    reference_id: Optional[UUID] = None

    @staticmethod
    def pack(view: language.DatasetView, tree: TreeP) -> "DatasetViewData":
        return DatasetViewData(
            id=view.id,
            order_key=view.order_key,
            name=view.name,
            query=view.query,
            sort=view.sort,
            reference_id=view.reference.id if view.reference else None,
        )

    def unpack(self, parent: language.Statement, tree: TreeP) -> language.DatasetView:
        return language.DatasetView(
            id=self.id,
            name=self.name,
            source=parent,
            query=self.query,
            sort=self.sort,
            reference=self.reference_id,
        )


@dataclass(slots=True)
class RecordData(ModuleNode, Ordered):
    PARENTS = {MOT.SYMBOL}

    revision: int
    data: Optional[typing.Any] = None

    def __str__(self):
        return f"{self.parent_id}:{self.order_key} {describe_type(self.data)}"

    def __repr__(self):
        return f"<{self.__class__.__name__} {str(self)}>"

    @staticmethod
    def pack(record: language.Record, tree: TreeP) -> "RecordData":
        return RecordData(
            id=record.id,
            order_key=record.order_key,
            revision=record.revision,
            data=record.data,
        )

    @staticmethod
    def unpack(record: "RecordData", parent: language.Statement, tree: TreeP) -> language.Record:
        return language.Record(
            id=record.id,
            name=parent.name,
            source=parent,
            revision=record.revision,
            data=record.data,
        )


# interp


@dataclass(slots=True)
class InterpData(ModuleNode):
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


@dataclass(slots=True)
class IssueData(ModuleNode):
    PARENTS: ClassVar[Parents] = {MOT.INTERP}

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


@dataclass(slots=True)
class RemoteObjectData:
    id: UUID
    sha512: str
    content_length: int
    content_type: str
    name: Optional[str]

    def deepcopy(self):
        return RemoteObjectData(**asdict(self))

    def __str__(self):
        return f"{self.id} {self.name} ({self.content_type}, {self.content_length} bytes)"

    def __repr__(self):
        return f"<RemoteObject {self}>"

    @staticmethod
    def pack(object: language.RemoteObject) -> "RemoteObjectData":
        return RemoteObjectData(
            id=object.id,
            name=object.name,
            sha512=object.sha512,
            content_type=object.content_type,
            content_length=object.content_length,
        )

    def unpack(self) -> language.RemoteObject:
        return language.RemoteObject(
            id=self.id,
            name=self.name,
            sha512=self.sha512,
            content_type=self.content_type,
            content_length=self.content_length,
        )


@dataclass(slots=True)
class SecretData:
    id: UUID
    sha512: str
    value: Optional[typing.Any] = None

    def __deepcopy__(self):
        return SecretData(**asdict(self))

    def __str__(self):
        return f"{self.id} ({self.sha512})"

    def __repr__(self):
        return f"<Secret {self}>"

    @staticmethod
    def pack(secret: language.Secret) -> "SecretData":
        return SecretData(
            id=secret.id,
            sha512=secret.sha512,
            value=secret.value,
        )

    def unpack(self) -> language.Secret:
        return language.Secret(
            id=self.id,
            sha512=self.sha512,
            value=self.value,
        )
