import enum
import typing
from dataclasses import asdict, dataclass, fields
from hashlib import md5
from typing import Optional, Union
from uuid import UUID, uuid5

import bench.language.const
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
from bench.language.type import ModuleReference
from bench.utils.func import describe_type
from bench.utils.serialize import from_dict, to_dict

#
# Stable, concise and flat language data structures for transit and storage.
# TODO @Performance: use an optimized and evolvable wire format
#


@dataclass(repr=False, slots=True)
class FieldData:
    id: UUID
    revision: int
    name: Optional[str]
    key: str
    tag: TypeTag
    hint: Optional[TypeHint]
    statement_id: UUID
    order_key: str
    description: Optional[str]
    flags: TypeFlag
    value: Optional[typing.Any] = None
    reference_id: Optional[UUID] = None
    metadata: Optional[typing.Any] = None

    def __str__(self):
        name_str = f"{self.name} " if self.name else ""
        return f"{self.statement_id}:{self.order_key} {name_str}{self.tag.value}"

    def __repr__(self):
        return f"<Field {str(self)}>"

    def deepcopy(self):
        return FieldData(
            id=self.id,
            revision=self.revision,
            name=self.name,
            key=self.key,
            tag=self.tag,
            hint=self.hint,
            statement_id=self.statement_id,
            order_key=self.order_key,
            description=self.description,
            flags=self.flags,
            reference_id=self.reference_id,
        )


@dataclass(repr=False, slots=True)
class XBlockData:
    kind: str
    source: str
    value: Optional[typing.Any] = None
    path: Optional[str] = None

    def __str__(self):
        return f"{self.kind} {self.source}"

    def __repr__(self):
        return f"<XBlock {str(self)}>"


@dataclass(repr=False, slots=True)
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


@dataclass(repr=False, slots=True)
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


@dataclass(repr=False, slots=True)
class ModuleData:
    id: UUID
    name: str
    files: list["FileData"]
    committed: bool = False

    def __str__(self):
        return f"{self.name}@{self.id} ({len(self.files)} files)"

    def __repr__(self):
        return f"<Module {str(self)}>"

    def deepcopy(self):
        return ModuleData(
            id=self.id,
            name=self.name,
            committed=self.committed,
            files=[f.deepcopy() for f in self.files],
        )


@dataclass(repr=False, slots=True)
class FileData:
    id: UUID
    module_id: UUID
    path: str
    statements: list["StatementData"]
    revision: int

    def __str__(self):
        return f"{self.module_id}/{self.path}"

    @property
    def name(self):
        # name property to emulate File model
        return self.path.split(".")[-1]

    def __repr__(self):
        return f"<File {str(self)}>"

    def deepcopy(self):
        return FileData(
            id=self.id,
            module_id=self.module_id,
            path=self.path,
            revision=self.revision,
            statements=[s.deepcopy() for s in self.statements],
        )


@dataclass(repr=False, slots=True)
class StatementData:
    id: UUID
    module_id: UUID
    file_id: UUID
    order_key: str
    revision: int
    type: StatementType
    modifier: Optional[StatementModifier]
    name: Optional[str]
    fqn: Optional[str]
    parent_id: Optional[UUID]
    text: Optional[str]
    # symbol contents
    symbol_type: Optional[SymbolType]
    description: Optional[str] = None
    _symbol: Optional[typing.Any] = None  # hack until we get proper wire formats

    def __post_init__(self):
        data_cls = SYMBOL_DATA_CLASS_BY_TYPE.get(self.symbol_type)
        if data_cls and self._symbol:
            self._symbol = from_dict(data_cls, self._symbol)

    @property
    def symbol(self):
        return self._symbol

    @symbol.setter
    def symbol(self, value):
        self._symbol = to_dict(value)

    def __str__(self):
        parent_str = f"{self.parent_id}:" if self.parent_id else ""
        loc = str(self.file_id) + ":" + parent_str + str(self.order_key)
        symbol_type_str = self.symbol_type.name if self.symbol_type else ""
        return f"{loc}: {self.type.name} {symbol_type_str} {self.name}"

    def __repr__(self):
        return f"<Statement {str(self)}>"

    def deepcopy(self):
        copied = {}
        for field in _STATEMENT_DATA_FIELDS:
            value = getattr(self, field.name)
            if value is not None and hasattr(value, "deepcopy"):
                copied[field.name] = value.deepcopy()
            else:
                copied[field.name] = value
        return StatementData(**copied)


_STATEMENT_DATA_FIELDS = fields(StatementData)


# symbols


@dataclass(repr=False, slots=True)
class HasTypeData:
    tag: Optional[TypeTag] = None
    flags: Optional[TypeFlag] = TypeFlag.Zero
    fields: Union[list[FieldData], None] = None


@dataclass(repr=False, slots=True)
class TypeData(HasTypeData):
    pass


@dataclass(repr=False, slots=True)
class TaskData(HasTypeData):
    pass


@dataclass(repr=False, slots=True)
class ExpectationData(HasTypeData):
    pass


@dataclass(repr=False, slots=True)
class CodeData(HasTypeData):
    lang: Optional[str] = None
    code: Optional[str] = None


@dataclass(repr=False, slots=True)
class ModelData:
    external_name: Optional[str] = None


@dataclass(repr=False, slots=True)
class RequirementData:
    reference_module: Optional[ModuleReference] = None


@dataclass(repr=False, slots=True)
class ValueData(HasTypeData):
    value: Optional[typing.Any] = None


@dataclass(repr=False, slots=True)
class DatasetViewData:
    id: UUID
    name: str
    query: Optional[Query] = None
    sort: Optional[list[Sort]] = None
    length: Optional[int] = None
    reference_id: Optional[UUID] = None


@dataclass(repr=False, slots=True)
class RecordData:
    id: UUID
    statement_id: UUID
    order_key: str
    revision: int
    data: Optional[typing.Any] = None

    def __str__(self):
        return f"{self.statement_id}:{self.order_key} {describe_type(self.data)}"

    def __repr__(self):
        return f"<{self.__class__.__name__} {str(self)}>"

    def deepcopy(self):
        return RecordData(**self.__dict__)


@dataclass(repr=False, slots=True)
class DatasetData(HasTypeData):
    records: Optional[list[RecordData]] = None
    length: Optional[int] = None

    @property
    def loaded(self):
        return self.records is not None


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


# interp


@dataclass(repr=False, slots=True)
class IssueData:
    id: UUID
    scope: InterpScope
    file_id: Optional[UUID]
    statement_id: Optional[UUID]
    kind: IssueKind
    type: IssueType
    message: Optional[str]


@dataclass(repr=False, slots=True)
class InterpData:
    scope: InterpScope
    file_id: Optional[UUID]
    statement_id: Optional[UUID]
    issues: Optional[list[IssueData]] = None
    resolved_fields: Optional[list[FieldData]] = None

    def hash_content(self) -> str:
        content = (
            self.statement_id,
            *(issue.id for issue in (self.issues or [])),
            *(field.id for field in (self.resolved_fields or [])),
        )
        content = str(content).encode("utf-8")
        return md5(content).hexdigest()


def pack_module(module: language.Module) -> ModuleData:
    return ModuleData(
        id=module.id,
        name=module.name,
        files=[pack_file(file) for file in module.files],
    )


def unpack_module(data: ModuleData) -> language.Module:
    """Maps module data back into a module. Restores explicit statement references without checking!"""
    module = language.Module(id=data.id, name=data.name)
    module.files = [unpack_file(file, module) for file in data.files]
    # replace parent references
    statements = {statement.id: statement for file in module.files for statement in file.statements}
    for file_data, file in zip(data.files, module.files):
        for statement_data, statement in zip(file_data.statements, file.statements):
            if statement_data.parent_id is not None:
                # parent should exist but may not if it was not included in the module data
                # but exists in the source (e.g. an invalid comment parent)
                statement.parent = statements.get(statement_data.parent_id)
    return module


def pack_file(file: language.File) -> FileData:
    return FileData(
        id=file.id,
        module_id=file.module.id,
        path=file.path,
        statements=[pack_statement(statement) for statement in file.statements],
        revision=1,
    )


def unpack_file(data: FileData, module: language.Module) -> language.File:
    file = language.File(
        id=data.id,
        module=module,
        path=data.path,
    )
    file.statements = [unpack_statement(statement, file) for statement in data.statements]
    return file


def pack_statement(statement: language.Statement) -> StatementData:
    """Maps a language statement to a wire statement (incl. refs)."""
    # use statement id if possible, else use statement path
    data = StatementData(
        module_id=statement.file.module.id,
        file_id=statement.file.id,
        revision=1,
        id=statement.id,
        order_key=statement.order_key,
        parent_id=statement.parent_id,
        type=statement.type,
        name=statement.name,
        fqn=statement.fqn,
        text=statement.text,
        symbol_type=statement.symbol_type,
    )
    if statement.symbol is not None:
        _pack_symbol(statement.symbol, data)
    return data


def unpack_statement(data: StatementData, file: language.File) -> language.Statement:
    """Maps a wire statement into the language representation"""
    statement = language.Statement(
        id=data.id,
        file=file,
        parent=None,  # must be restored later
        order_key=data.order_key,
        type=data.type,
        name=data.name,
        text=data.text,
    )
    if statement.type == bench.language.const.StatementType.SYMBOL:
        base_args = dict(
            id=data.id,
            name=data.name,
            modifier=data.modifier,
            source=statement,
            scope=statement,
        )
        statement.symbol = _unpack_symbol(data, base_args)
        statement.symbol.source = statement
    return statement


def _pack_symbol(symbol: language.Symbol, data: StatementData) -> None:
    """Maps a language symbol's to a wire statement."""
    # type content is also a component
    data.modifier = symbol.modifier
    if isinstance(symbol, language.TypeNode):
        data.description = symbol.description
        data.root_type_tag = symbol.tag
        data.root_type_flags = symbol.flags
        data.fields = [pack_field(data.id, node) for node in symbol.fields]
    if isinstance(symbol, language.Task):
        data.description = symbol.description
    elif isinstance(symbol, language.Expectation):
        data.description = symbol.description
    elif isinstance(symbol, language.Code):
        data.description = symbol.description
        data.lang = symbol.language
        data.code = symbol.code
    elif isinstance(symbol, language.Model):
        data.external_name = symbol.external_name
    elif isinstance(symbol, language.Dataset):
        data.description = symbol.description
    elif isinstance(symbol, language.Requirement):
        if symbol.module_name and symbol.version:
            data.reference_module = ModuleReference(
                symbol.module_name, symbol.version, id=symbol.module_id
            )


def _unpack_symbol(data: StatementData, base_args: dict) -> language.Symbol:
    """Maps a wire statement's symbol to a language symbol."""
    if isinstance(data.symbol, TypeData):
        return language.Type(
            **base_args,
            description=data.description,
            tag=data.symbol.tag,
            fields=[unpack_field(node) for node in data.symbol.fields],
        )
    elif isinstance(data.symbol, TaskData):
        return language.Task(
            **base_args,
            description=data.description,
            tag=data.symbol.tag,
            fields=[unpack_field(node) for node in data.symbol.fields],
        )
    elif isinstance(data.symbol, ExpectationData):
        return language.Expectation(**base_args, description=data.description)
    elif isinstance(data.symbol, CodeData):
        return language.Code(
            **base_args,
            description=data.description,
            language=data.symbol.lang,
            code=data.symbol.code,
            fields=[unpack_field(node) for node in data.symbol.fields],
        )
    elif isinstance(data.symbol, ModelData):
        return language.Model(
            **base_args,
            external_name=data.symbol.external_name,
        )
    elif isinstance(data.symbol, DatasetData):
        return language.Dataset(
            **base_args,
            description=data.description,
            fields=[unpack_field(node) for node in data.symbol.fields],
            records=[unpack_record(r) for r in data.symbol.records]
            if data.symbol.records is not None
            else None,
        )
    elif isinstance(data.symbol, RequirementData):
        if data.symbol.reference_module is not None:
            return language.Requirement(
                **base_args,
                module_name=data.symbol.reference_module.name,
                version=data.symbol.reference_module.version,
                module_id=data.symbol.reference_module.id,
            )
        else:
            return language.Requirement(**base_args)
    else:
        raise ValueError(f"unexpected symbol type {data.symbol_type} for statement {data}")


def pack_field(statement_id: UUID, node: language.Field) -> FieldData:
    """Maps a field to a field data object."""
    return FieldData(
        id=node.id,
        revision=1,
        name=node.name,
        key=node.key,
        statement_id=statement_id,
        tag=node.tag,
        hint=node.hint,
        description=node.description,
        flags=node.flags,
        reference_id=node.reference.id if hasattr(node.reference, "id") else node.reference,
        order_key=node.order_key,
    )


def unpack_field(node: FieldData) -> language.Field:
    """Maps a field data object to a field."""
    return language.Field(
        id=node.id,
        name=node.name,
        key=node.key,
        tag=node.tag,
        hint=node.hint,
        description=node.description,
        flags=node.flags,
        reference=node.reference_id,
        order_key=node.order_key,
    )


def pack_record(statement_id: UUID, record: language.Record) -> RecordData:
    """Maps a record to a record data object."""
    return RecordData(
        id=record._id,
        revision=1,
        data=record._data,
        statement_id=statement_id,
        order_key=record._order_key,
    )


def unpack_record(data: RecordData) -> language.Record:
    """Maps a record data object to a record."""
    return language.Record(_id=data.id, _data=data.data, _order_key=data.order_key)


def pack_issue(issue: language.Issue) -> IssueData:
    if issue.subject is not None:
        id = uuid5(issue.subject.id, issue.type.name)
    else:
        raise NotImplementedError(f"cannot handle unscoped issue: {issue}")
    return IssueData(
        id=id,
        kind=IssueKind.ERROR,
        scope=issue.scope,
        type=issue.type,
        file_id=issue.subject.id if isinstance(issue.subject, language.File) else None,
        statement_id=issue.subject.id if isinstance(issue.subject, language.Statement) else None,
        message=issue.message,
    )


def pack_remote_object(object: language.RemoteObject) -> RemoteObjectData:
    return RemoteObjectData(
        id=object.id,
        name=object.name,
        sha512=object.sha512,
        content_type=object.content_type,
        content_length=object.content_length,
    )


def unpack_remote_object(object: RemoteObjectData) -> language.RemoteObject:
    return language.RemoteObject(
        id=object.id,
        name=object.name,
        sha512=object.sha512,
        content_type=object.content_type,
        content_length=object.content_length,
    )


def pack_secret(secret: language.Secret) -> SecretData:
    return SecretData(
        id=secret.id,
        sha512=secret.sha512,
        value=secret.value,
    )


def unpack_secret(secret: SecretData) -> language.Secret:
    return language.Secret(
        id=secret.id,
        sha512=secret.sha512,
        value=secret.value,
    )


#
# Executions
#


class ExecutionTriggerType(enum.StrEnum):
    API = "rest"
    UI = "ui"
    REACTIVE = "reactive"
    SCHEDULED = "scheduled"
