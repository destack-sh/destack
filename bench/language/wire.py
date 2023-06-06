import enum
import typing
from dataclasses import asdict, dataclass, fields
from hashlib import md5
from typing import Optional, Union
from uuid import UUID, uuid5

from bench import language
from bench.language import IssueType
from bench.language.interp import get_reference_as_path
from bench.language.type import (
    StatementModifier,
    StatementPath,
    StatementType,
    SymbolType,
    TypeFlag,
    TypeHint,
    TypeTag,
    XKind,
    XSource,
)
from bench.utils.func import describe_type

#
# Stable, concise and flat language data structures for transit and storage.
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
    reference_id: Union[None, UUID] = None

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
class XBlockData:
    kind: XKind
    source: XSource
    value: Optional[typing.Any] = None
    path: Optional[str] = None

    def __str__(self):
        return f"{self.kind.value} {self.source.value}"

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
    generated: bool

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
            generated=self.generated,
            statements=[s.deepcopy() for s in self.statements],
        )


ModuleReference = typing.NamedTuple(
    "ModuleReference", [("name", str), ("version", str), ("id", Optional[UUID])]
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
    reference: Union[None, StatementPath, UUID]
    text: Optional[str]
    symbol_type: Optional[SymbolType]
    generated: bool
    # symbol contents
    root_type_tag: Optional[TypeTag] = None
    root_type_flags: Optional[TypeFlag] = None
    fields: Union[list[FieldData], None] = None
    description: Optional[str] = None
    lang: Optional[str] = None
    code: Optional[str] = None
    provider: Optional[str] = None
    external_name: Optional[str] = None
    records: Optional[list[RecordData]] = None
    reference_module: Optional[ModuleReference] = None

    @property
    def reference_id(self) -> Optional[UUID]:
        return self.reference if isinstance(self.reference, UUID) else None

    def __str__(self):
        parent_str = f"{self.parent_id}:" if self.parent_id else ""
        loc = str(self.file_id) + ":" + parent_str + str(self.order_key)
        symbol_type_str = self.symbol_type.name if self.symbol_type else ""
        ref_str = f"ref={self.reference}" if self.reference else ""
        ref_module_str = f"ref_module={self.reference_module}" if self.reference_module else ""
        content_str = ", ".join((s for s in (ref_str, ref_module_str) if s))
        return f"{loc}: {self.type.name} {symbol_type_str} {self.name} ({content_str})"

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


class InterpScope(enum.StrEnum):
    MODULE = "module"
    FILE = "file"
    STATEMENT = "statement"


class IssueKind(enum.StrEnum):
    ERROR = "error"
    WARNING = "warning"
    SUGGESTION = "suggestion"


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


def rmap_module(module: language.Module, impute_type_references: bool = False) -> ModuleData:
    return ModuleData(
        id=module.id,
        name=module.name,
        files=[
            rmap_file(file, impute_type_references=impute_type_references) for file in module.files
        ],
    )


def wmap_module(data: ModuleData) -> language.Module:
    """Maps module data back into a module. Restores explicit statement references without checking!"""
    module = language.Module(id=data.id, name=data.name)
    module.files = [wmap_file(file, module) for file in data.files]

    # TODO @Architecture @Cleanup: remove reference resolution as part of wire mapping
    #  It shouldn't be needed anymore, even now, but getting some errors, so fix later.
    # restore statement references
    statements = {statement.id: statement for file in module.files for statement in file.statements}
    for file_data, file in zip(data.files, module.files):
        for statement_data, statement in zip(file_data.statements, file.statements):
            if statement_data.parent_id is not None:
                # parent should exist but may not if it was not included in the module data
                # but exists in the source (e.g. an invalid comment parent)
                statement.parent = statements.get(statement_data.parent_id)

            # resolve references in statement and in symbol content
            # ignore references we couldn't find since are either
            #  1) refs to "deleted" statements or
            #  2) refs to statements in other modules (which should be by path anyway, but we can't check here)

            if isinstance(statement_data.reference, UUID):
                reference = statements.get(statement_data.reference, None)
                if reference is not None:
                    statement.reference = get_reference_as_path(reference, statement)
    return module


def rmap_file(file: language.File, impute_type_references: bool = False) -> FileData:
    return FileData(
        id=file.id,
        module_id=file.module.id,
        path=file.path,
        generated=file.generated,
        statements=[
            rmap_statement(statement, impute_type_references=impute_type_references)
            for statement in file.statements
        ],
        revision=1,
    )


def wmap_file(data: FileData, module: language.Module) -> language.File:
    file = language.File(
        id=data.id,
        module=module,
        path=data.path,
        generated=data.generated,
    )
    file.statements = [wmap_statement(statement, file) for statement in data.statements]
    return file


def rmap_statement(
    statement: language.Statement, impute_type_references: bool = False
) -> StatementData:
    """Maps a language statement to a wire statement (incl. refs)."""
    # use statement id if possible, else use statement path
    reference = (
        statement.reference_id if statement.reference_id is not None else statement.reference
    )
    data = StatementData(
        module_id=statement.file.module.id,
        file_id=statement.file.id,
        revision=1,
        id=statement.id,
        order_key=statement.order_key,
        parent_id=statement.parent_id,
        type=statement.type,
        modifier=statement.modifier,
        reference=reference,
        name=statement.name,
        fqn=statement.fqn,
        text=statement.text,
        symbol_type=statement.symbol_type,
        generated=statement.generated,
    )
    if statement.content is not None:
        rmap_symbol(statement.content, data, impute_type_references=impute_type_references)
    return data


def wmap_statement(data: StatementData, file: language.File) -> language.Statement:
    """Maps a wire statement's _contents_ (excl. refs) to a language statement."""
    reference = data.reference if isinstance(data.reference, (StatementPath, UUID)) else None
    statement = language.Statement(
        id=data.id,
        file=file,
        parent=None,  # must be restored later
        reference=reference,  # also restored later if it was an id
        order_key=data.order_key,
        type=data.type,
        modifier=data.modifier,
        name=data.name,
        text=data.text,
        symbol_type=data.symbol_type,
        generated=data.generated,
    )
    if statement.type == StatementType.DEFINITION:
        statement.content = wmap_symbol(data)
    return statement


def rmap_symbol(
    content: language.SymbolContent, data: StatementData, impute_type_references: bool = False
) -> None:
    """Maps a language symbol's _contents_ (excl. refs) to a wire statement."""
    # type content is also a component
    if isinstance(content, language.TypeContent):
        data.description = content.description
        data.root_type_tag = content.tag
        data.root_type_flags = content.flags
        source_nodes = (
            content.fields if impute_type_references else (content.fields or content.fields)
        )
        data.fields = [rmap_field(data.id, node, impute_type_references) for node in source_nodes]
    if isinstance(content, language.TaskContent):
        data.description = content.description
    elif isinstance(content, language.ExpectationContent):
        data.description = content.description
    elif isinstance(content, language.CodeContent):
        data.description = content.description
        data.lang = content.language
        data.code = content.code
    elif isinstance(content, language.ModelContent):
        data.external_name = content.external_name
    elif isinstance(content, language.CapabilityContent):
        data.description = content.description
    elif isinstance(content, language.DataContent):
        data.lang = content.language
        data.description = content.description
        data.records = [rmap_record(data.id, r) for r in content.records]
    elif isinstance(content, language.BuildContent):
        data.description = content.comment
    elif isinstance(content, language.RequirementContent):
        if content.module_name and content.version:
            data.reference_module = ModuleReference(
                content.module_name, content.version, id=content.module_id
            )


def wmap_symbol(data: StatementData) -> language.SymbolContent:
    """Maps a wire statement's symbol contents to a language symbol."""
    if data.root_type_tag:
        fields = [wmap_field(t) for t in (data.fields or [])]
    else:
        fields = []

    if data.symbol_type == SymbolType.TYPE:
        return language.TypeContent(
            name=data.name,
            description=data.description,
            tag=data.root_type_tag,
            fields=fields,
        )
    elif data.symbol_type == SymbolType.TASK:
        return language.TaskContent(
            tag=data.root_type_tag,
            fields=fields,
            description=data.description,
        )
    elif data.symbol_type == SymbolType.EXPECTATION:
        return language.ExpectationContent(description=data.description)
    elif data.symbol_type == SymbolType.CODE:
        return language.CodeContent(
            description=data.description,
            language=data.lang,
            code=data.code,
            tag=data.root_type_tag,
            fields=fields,
        )
    elif data.symbol_type == SymbolType.MODEL:
        return language.ModelContent(
            external_name=data.external_name,
        )
    elif data.symbol_type == SymbolType.CAPABILITY:
        return language.CapabilityContent(description=data.description)
    elif data.symbol_type == SymbolType.DATA:
        return language.DataContent(
            description=data.description,
            language=data.lang,
            tag=data.root_type_tag,
            fields=fields,
            flags=data.root_type_flags,
            records=[wmap_record(r) for r in (data.records or [])],
        )
    elif data.symbol_type == SymbolType.BUILD:
        return language.BuildContent(comment=data.description)
    elif data.symbol_type == SymbolType.REQUIREMENT:
        return language.RequirementContent(
            module_name=data.reference_module.name if data.reference_module else None,
            version=data.reference_module.version if data.reference_module else None,
            module_id=data.reference_module.id if data.reference_module else None,
        )
    else:
        raise ValueError(f"unexpected symbol type {data.symbol_type} for statement {data}")


def rmap_field(statement_id: UUID, node: language.Field, impute_type_references: bool) -> FieldData:
    """Maps a field to a field data object."""
    has_reference = isinstance(node.reference, language.TypeContent)
    return FieldData(
        id=node.id,
        revision=1,
        name=node.name,
        key=node.key,
        statement_id=statement_id,
        tag=node.reference.tag if has_reference and impute_type_references else node.tag,
        hint=node.reference.hint if has_reference and impute_type_references else node.hint,
        description=node.description,
        flags=node.flags,
        reference_id=node.reference.id if hasattr(node.reference, "id") else node.reference,
        order_key=node.order_key,
    )


def wmap_field(node: FieldData) -> language.Field:
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


def rmap_record(statement_id: UUID, record: language.Record) -> RecordData:
    """Maps a record to a record data object."""
    return RecordData(
        id=record.id,
        revision=1,
        data=record.data,
        statement_id=statement_id,
        order_key=record.order_key,
    )


def wmap_record(data: RecordData) -> language.Record:
    """Maps a record data object to a record."""
    return language.Record(id=data.id, data=data.data, order_key=data.order_key)


def wmap_xblock(xblock: XBlockData) -> language.XBlockContent:
    """Maps an xblock data object to an xblock."""
    return language.XBlockContent(
        kind=xblock.kind,
        source=xblock.source,
        value=xblock.value,
        path=xblock.path,
    )


def rmap_xblock(xblock: language.XBlockContent) -> XBlockData:
    """Maps an xblock to an xblock data object."""
    return XBlockData(
        kind=xblock.kind,
        source=xblock.source,
        value=xblock.value,
        path=xblock.path,
    )


def rmap_issue(issue: language.Error) -> IssueData:
    if issue.statement is not None:
        id = uuid5(issue.statement.id, issue.type.name)
        scope = InterpScope.STATEMENT
    elif issue.file is not None:
        id = uuid5(issue.file.id, issue.type.name)
        scope = InterpScope.FILE
    else:
        raise ValueError(f"cannot handle unscoped issue yet: {issue}")
    return IssueData(
        id=id,
        kind=IssueKind.ERROR,
        scope=scope,
        type=issue.type,
        file_id=issue.file.id if issue.file else None,
        statement_id=issue.statement.id if issue.statement else None,
        message=issue.message,
    )


def rmap_remote_object(object: language.RemoteObject) -> RemoteObjectData:
    return RemoteObjectData(
        id=object.id,
        name=object.name,
        sha512=object.sha512,
        content_type=object.content_type,
        content_length=object.content_length,
    )


def wmap_remote_object(object: RemoteObjectData) -> language.RemoteObject:
    return language.RemoteObject(
        id=object.id,
        name=object.name,
        sha512=object.sha512,
        content_type=object.content_type,
        content_length=object.content_length,
    )


def rmap_secret(secret: language.Secret) -> SecretData:
    return SecretData(
        id=secret.id,
        sha512=secret.sha512,
        value=secret.value,
    )


def wmap_secret(secret: SecretData) -> language.Secret:
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
