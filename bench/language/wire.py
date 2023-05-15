import enum
import typing
from dataclasses import dataclass
from typing import Optional, Union
from uuid import UUID

from bench import language
from bench.language import ErrorType
from bench.language.parse import get_reference_as_path
from bench.language.type import (
    BuildSettings,
    GeneratedMapping,
    StatementModifier,
    StatementPath,
    StatementType,
    SymbolType,
    TypeFlag,
    TypeTag,
    XKind,
    XSource,
)
from bench.utils.func import describe_type

#
# Stable, concise and flat language data structures for transit and storage.
#


@dataclass(repr=False, slots=True)
class SimpleTypeNodeData:
    id: UUID
    revision: int
    name: Optional[str]
    key: str
    tag: TypeTag
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
        return f"<SimpleTypeNode {str(self)}>"


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


@dataclass(repr=False, slots=True)
class XBlockData:
    id: UUID
    statement_id: UUID
    order_key: str
    kind: XKind
    source: XSource
    revision: int
    value: Optional[typing.Any] = None
    path: Optional[str] = None
    description: Optional[str] = None

    def __str__(self):
        return f"{self.order_key} {self.kind.value} {self.source.value}"

    def __repr__(self):
        return f"<XBlock {str(self)}>"


@dataclass(repr=False, slots=True)
class RemoteObjectData:
    id: UUID
    sha512: str
    content_length: int
    content_type: str
    name: Optional[str]


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
    type_nodes: Union[list[SimpleTypeNodeData], None] = None
    description: Optional[str] = None
    lang: Optional[str] = None
    code: Optional[str] = None
    xblocks: Optional[list[XBlockData]] = None
    provider: Optional[str] = None
    external_name: Optional[str] = None
    records: Optional[list[RecordData]] = None
    generated_mappings: Optional[list[GeneratedMapping]] = None
    build_settings: Optional[BuildSettings] = None
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
    # generator content is a component of other content types
    if isinstance(content, language.GeneratorContent):
        data.generated_mappings = content.generated_mappings
    # type content is also a component
    if isinstance(content, language.TypeContent):
        data.description = content.description
        data.root_type_tag = content.tag
        data.root_type_flags = content.flags
        data.type_nodes = [
            rmap_simple_type_node(data.id, node, impute_type_references)
            for node in content.type_nodes
        ]
    if isinstance(content, language.TaskContent):
        data.description = content.description
    elif isinstance(content, language.ExpectationContent):
        data.description = content.description
    elif isinstance(content, language.CodeContent):
        data.description = content.description
        data.lang = content.language
        data.code = content.code
        data.xblocks = [rmap_xblock(data.id, xblock) for xblock in content.xblocks]
    elif isinstance(content, language.ModelContent):
        data.provider = content.provider
        data.external_name = content.external_name
    elif isinstance(content, language.CapabilityContent):
        data.description = content.description
    elif isinstance(content, language.DataContent):
        data.lang = content.language
        data.description = content.description
        data.records = [rmap_record(data.id, r) for r in content.records]
    elif isinstance(content, language.BuildContent):
        data.description = content.comment
        data.build_settings = content.settings
    elif isinstance(content, language.RequirementContent):
        if content.module_name and content.version:
            data.reference_module = ModuleReference(
                content.module_name, content.version, id=content.module_id
            )


def wmap_symbol(data: StatementData) -> language.SymbolContent:
    """Maps a wire statement's symbol contents to a language symbol."""
    if data.root_type_tag:
        type_nodes = [wmap_simple_type_node(t) for t in (data.type_nodes or [])]
    else:
        type_nodes = []

    if data.symbol_type == SymbolType.TYPE:
        return language.TypeContent(
            name=data.name,
            description=data.description,
            tag=data.root_type_tag,
            type_nodes=type_nodes,
        )
    elif data.symbol_type == SymbolType.TASK:
        return language.TaskContent(
            generated_mappings=data.generated_mappings,
            tag=data.root_type_tag,
            type_nodes=type_nodes,
            description=data.description,
        )
    elif data.symbol_type == SymbolType.EXPECTATION:
        return language.ExpectationContent(description=data.description)
    elif data.symbol_type == SymbolType.CODE:
        return language.CodeContent(
            generated_mappings=data.generated_mappings,
            description=data.description,
            language=data.lang,
            code=data.code,
            tag=data.root_type_tag,
            type_nodes=type_nodes,
            xblocks=[wmap_xblock(x) for x in (data.xblocks or [])],
        )
    elif data.symbol_type == SymbolType.MODEL:
        return language.ModelContent(
            provider=data.provider,
            external_name=data.external_name,
        )
    elif data.symbol_type == SymbolType.CAPABILITY:
        return language.CapabilityContent(description=data.description)
    elif data.symbol_type == SymbolType.DATA:
        return language.DataContent(
            description=data.description,
            language=data.lang,
            tag=data.root_type_tag,
            type_nodes=type_nodes,
            flags=data.root_type_flags,
            records=[wmap_record(r) for r in (data.records or [])],
        )
    elif data.symbol_type == SymbolType.BUILD:
        return language.BuildContent(
            comment=data.description,
            generated_mappings=data.generated_mappings,
            settings=data.build_settings,
        )
    elif data.symbol_type == SymbolType.REQUIREMENT:
        return language.RequirementContent(
            module_name=data.reference_module.name if data.reference_module else None,
            version=data.reference_module.version if data.reference_module else None,
            module_id=data.reference_module.id if data.reference_module else None,
        )
    else:
        raise ValueError(f"unexpected symbol type {data.symbol_type} for statement {data}")


def rmap_simple_type_node(
    statement_id: UUID, node: language.SimpleTypeNode, impute_type_references: bool
) -> SimpleTypeNodeData:
    """Maps a simple type node to a simple type node data object."""
    return SimpleTypeNodeData(
        id=node.id,
        revision=1,
        name=node.name,
        key=node.key,
        statement_id=statement_id,
        tag=node.reference.tag if node.reference and impute_type_references else node.tag,
        description=node.description,
        flags=node.flags,
        reference_id=node.reference.id if node.reference else None,
        order_key=node.order_key,
        value=node.value,
    )


def wmap_simple_type_node(node: SimpleTypeNodeData) -> language.SimpleTypeNode:
    """Maps a simple type node data object to a simple type node."""
    return language.SimpleTypeNode(
        id=node.id,
        name=node.name,
        key=node.key,
        tag=node.tag,
        description=node.description,
        flags=node.flags,
        reference=node.reference_id,
        order_key=node.order_key,
        value=node.value,
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
        id=xblock.id,
        order_key=xblock.order_key,
        kind=xblock.kind,
        source=xblock.source,
        value=xblock.value,
        path=xblock.path,
        description=xblock.description,
    )


def rmap_xblock(statement_id: UUID, xblock: language.XBlockContent) -> XBlockData:
    """Maps an xblock to an xblock data object."""
    return XBlockData(
        id=xblock.id,
        statement_id=statement_id,
        order_key=xblock.order_key,
        kind=xblock.kind,
        source=xblock.source,
        value=xblock.value,
        path=xblock.path,
        description=xblock.description,
        revision=1,
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


#
# Errors
#


@dataclass(repr=False)
class ErrorData:
    type: ErrorType
    statement_id: Optional[UUID]
    message: str
    verbose_message: Optional[str]

    def __str__(self):
        return f"{self.type.name}: {self.message}"

    def __repr__(self):
        return f"<Error {str(self)}>"


def rmap_error(error: language.Error) -> ErrorData:
    return ErrorData(
        type=error.type,
        statement_id=error.statement.id if error.statement is not None else None,
        message=error.message,
        verbose_message=error.verbose_message,
    )


# TODO @Cleanup: execution data doesn't belong to language wire format

#
# Executions
#


class ExecutionTriggerType(enum.StrEnum):
    REST_API = "rest-api"
    UI_INTERACTIVE = "ui-interactive"
    JOB = "job"
    MANUAL = "manual"


class ExecutionTracingLevel(enum.StrEnum):
    ROOT_FRAME = "root-frame"
    ROOT_FRAME_WITH_DATA = "root-frame-with-data"
    ALL_FRAMES = "all-frames"
    ALL_FRAMES_WITH_DATA = "all-frames-with-data"
