import enum
import typing
from collections import OrderedDict
from dataclasses import dataclass
from typing import Optional, Union
from uuid import UUID

from more_itertools import first

from bench import language
from bench.language import ErrorType
from bench.language.reconstruct import get_reference_as_path
from bench.language.type import (
    GeneratedMapping,
    LiteralValue,
    StatementModifier,
    StatementPath,
    StatementType,
    SymbolType,
    TypeTag,
    XKind,
    XSource,
)
from bench.utils.fractional import INTEGER_ZERO, generate_n_keys_between

#
# Stable, concise and flat language data structures for transit and storage.
#


@dataclass(repr=False, slots=True)
class TypeNodeData:
    id: UUID
    revision: int
    name: Optional[str]
    tag: TypeTag
    order_key: str
    description: Optional[str] = None
    value: Optional[typing.Any] = None
    reference: Union[None, StatementPath, UUID] = None
    parent_id: Optional[UUID] = None

    def __str__(self):
        name_str = f"{self.name} " if self.name else ""
        return f"{name_str}{self.tag.value}"

    def __repr__(self):
        return f"<TypeNode {str(self)}>"


@dataclass(repr=False, slots=True)
class RecordData:
    id: UUID
    order_key: str
    revision: int
    data: Optional[typing.Any] = None


@dataclass(repr=False, slots=True)
class XBlockData:
    id: UUID
    order_key: str
    kind: XKind
    source: XSource
    value: Optional[typing.Any] = None
    path: Optional[str] = None
    description: Optional[str] = None


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
        generated_str = ".gen" if self.generated else ""
        return f"{self.module_id}/{self.path}{generated_str}"

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
    type_nodes: Union[list[TypeNodeData], None] = None
    description: Optional[str] = None
    lang: Optional[str] = None
    code: Optional[str] = None
    xblocks: Optional[list[XBlockData]] = None
    provider: Optional[str] = None
    external_name: Optional[str] = None
    records: Optional[list[RecordData]] = None
    generated_mappings: Optional[list[GeneratedMapping]] = None
    value: LiteralValue = None
    on: Optional[str] = None
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


def rmap_module(module: language.Module) -> ModuleData:
    return ModuleData(
        id=module.id,
        name=module.name,
        files=[rmap_file(file) for file in module.files],
    )


def wmap_module(data: ModuleData) -> language.Module:
    """Maps module data back into a module. Restores explicit statement references without checking!"""
    module = language.Module(id=data.id, name=data.name)
    module.files = [wmap_file(file, module) for file in data.files]

    # restore statement references
    statements = {statement.id: statement for file in module.files for statement in file.statements}
    for data_file, file in zip(data.files, module.files):
        for data_statement, statement in zip(data_file.statements, file.statements):
            if data_statement.parent_id is not None:
                # parent should exist but may not if it was not included in the module data
                # but exists in the source (e.g. an invalid comment parent)
                statement.parent = statements.get(data_statement.parent_id)

            # resolve references in statement and in symbol content
            # ignore references we couldn't find since are either
            #  1) refs to "deleted" statements or
            #  2) refs to statements in other modules (which should be by path anyway, but we can't check here)

            if isinstance(data_statement.reference, UUID):
                reference = statements.get(data_statement.reference, None)
                if reference is not None:
                    statement.reference = get_reference_as_path(reference, statement)

            type_node = None
            if isinstance(statement.content, language.TypeNode):
                type_node = statement.content
            elif isinstance(
                statement.content,
                (language.DatasetContent, language.TaskContent, language.CodeContent),
            ):
                type_node = statement.content.type_node
            if type_node is not None:
                for node in type_node.walk():
                    data_node = first(n for n in data_statement.type_nodes if n.id == node.id)
                    if isinstance(data_node.reference, UUID):
                        reference = statements.get(node.reference, None)
                        if reference is not None:
                            node.reference = get_reference_as_path(reference, statement)
                            node.source_reference = node.reference

    return module


def rmap_file(file: language.File) -> FileData:
    return FileData(
        id=file.id,
        module_id=file.module.id,
        path=file.path,
        generated=file.generated,
        statements=[rmap_statement(statement) for statement in file.statements],
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


def rmap_statement(statement: language.Statement) -> StatementData:
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
        rmap_symbol(statement.content, data)
    return data


def wmap_statement(data: StatementData, file: language.File) -> language.Statement:
    """Maps a wire statement's _contents_ (excl. refs) to a language statement."""
    reference = data.reference if isinstance(data.reference, StatementPath) else None
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


def rmap_symbol(content: language.SymbolContent, data: StatementData) -> None:
    """Maps a language symbol's _contents_ (excl. refs) to a wire statement."""
    if isinstance(content, language.TypeNode):
        data.description = content.description
        data.type_nodes = rmap_type_node(content)
    elif isinstance(content, language.TaskContent):
        data.description = content.description
        data.type_nodes = rmap_type_node(content.type_node)
    elif isinstance(content, language.ExpectationContent):
        data.description = content.description
        data.on = content.on
    elif isinstance(content, language.CodeContent):
        data.description = content.description
        data.lang = content.language
        data.code = content.code
        data.type_nodes = rmap_type_node(content.type_node)
        data.xblocks = [rmap_xblock(xblock) for xblock in content.xblocks]
    elif isinstance(content, language.ModelContent):
        data.provider = content.provider
        data.external_name = content.external_name
    elif isinstance(content, language.ValueContent):
        data.description = content.description
        data.value = content.value
    elif isinstance(content, language.CapabilityContent):
        data.description = content.description
    elif isinstance(content, language.DatasetContent):
        data.lang = content.language
        data.description = content.description
        data.records = [rmap_record(r) for r in content.records]
        data.type_nodes = rmap_type_node(content.type_node)
    elif isinstance(content, language.BuildContent):
        data.generated_mappings = content.source_mappings
    elif isinstance(content, language.RequirementContent):
        if content.module_name and content.version:
            data.reference_module = ModuleReference(content.module_name, content.version, id=None)
    elif isinstance(content, language.RunconfigContent):
        pass
    else:
        raise ValueError(f"unexpected symbol type {content}")


def wmap_symbol(data: StatementData) -> language.SymbolContent:
    """Maps a wire statement's symbol contents to a language symbol."""
    if data.symbol_type == SymbolType.TYPE:
        type_node = wmap_type_node(data.type_nodes)
        type_node.description = data.description  # prefer type node from wire
        return type_node
    elif data.symbol_type == SymbolType.TASK:
        return language.TaskContent(
            type_node=wmap_type_node(data.type_nodes), description=data.description
        )
    elif data.symbol_type == SymbolType.EXPECTATION:
        return language.ExpectationContent(description=data.description, on=data.on)
    elif data.symbol_type == SymbolType.CODE:
        return language.CodeContent(
            description=data.description,
            language=data.lang,
            code=data.code,
            type_node=wmap_type_node(data.type_nodes),
            xblocks=[wmap_xblock(x) for x in data.xblocks],
        )
    elif data.symbol_type == SymbolType.MODEL:
        return language.ModelContent(
            provider=data.provider,
            external_name=data.external_name,
        )
    elif data.symbol_type == SymbolType.VALUE:
        return language.ValueContent(description=data.description, value=data.value)
    elif data.symbol_type == SymbolType.CAPABILITY:
        return language.CapabilityContent(description=data.description)
    elif data.symbol_type == SymbolType.DATA:
        return language.DatasetContent(
            description=data.description,
            language=data.lang,
            type_node=wmap_type_node(data.type_nodes),
            records=[wmap_record(r) for r in data.records],
        )
    elif data.symbol_type == SymbolType.BUILD:
        return language.BuildContent(source_mappings=data.generated_mappings)
    elif data.symbol_type == SymbolType.REQUIREMENT:
        return language.RequirementContent(
            module_name=data.reference_module.name if data.reference_module else None,
            version=data.reference_module.version if data.reference_module else None,
        )
    elif data.symbol_type == SymbolType.RUNCONFIG:
        return language.RunconfigContent()
    else:
        raise ValueError(f"unexpected symbol type {data.symbol_type} for statement {data}")


def wmap_type_node(nodes_data: list[TypeNodeData]) -> language.TypeNode:
    """Maps a flat list of wire type nodes to a language type node tree."""

    nodes_by_id = {}
    for data in nodes_data:
        if data.id in nodes_by_id:
            raise ValueError(f"duplicate type node id {data.id}: {data} and {nodes_by_id[data.id]}")
        node = language.TypeNode(
            id=data.id,
            name=data.name,
            tag=data.tag,
            description=data.description,
            value=data.value,
            reference=data.reference,
            source_reference=data.reference,
        )
        nodes_by_id[data.id] = node

    # assign children based on parent ids (sorted by order keys, which works per-parent)
    for data in sorted(nodes_data, key=lambda n: n.order_key):
        if data.parent_id is not None:
            parent = nodes_by_id[data.parent_id]
            if parent.children is None:
                parent.children = []
            parent.children.append(nodes_by_id[data.id])

    # find original root (the one with no parent)
    root = first(nodes_by_id.values(), lambda n: n.parent_id is None)
    return root


def rmap_record(record: language.Record) -> RecordData:
    """Maps a record to a record data object."""
    return RecordData(
        id=record.id,
        revision=1,
        data=record.data,
        order_key=record.order_key,
    )


def wmap_record(data: RecordData) -> language.Record:
    """Maps a record data object to a record."""
    return language.Record(id=data.id, data=data.data, order_key=data.order_key)


def rmap_type_node(node: language.TypeNode) -> list[TypeNodeData]:
    """Maps a type node tree structure to a flat list of type node data."""
    nodes_data = OrderedDict()
    for n in node.walk():
        if n.id in nodes_data:
            # already seen (multiple references to same node)
            # this is allowed in rmap but in wmap as TypeNodeData is flattened
            continue
        reference = n.reference
        if isinstance(reference, (language.TypeNode, language.Type)):
            reference = reference.id
        nodes_data[n.id] = TypeNodeData(
            id=n.id,
            revision=1,
            name=n.name,
            tag=n.tag,
            description=n.description,
            value=n.value,
            reference=reference,
            parent_id=None,  # will be set in second pass
            order_key=INTEGER_ZERO,  # will be set in second pass
        )

    # assign parent ids
    for n in node.walk():
        if n.children is not None:
            child_order_keys = generate_n_keys_between(None, None, len(n.children))
            for order_key, child in zip(child_order_keys, n.children):
                nodes_data[child.id].order_key = order_key
                nodes_data[child.id].parent_id = n.id

    return list(nodes_data.values())


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


def rmap_xblock(xblock: language.XBlockContent) -> XBlockData:
    """Maps an xblock to an xblock data object."""
    return XBlockData(
        id=xblock.id,
        order_key=xblock.order_key,
        kind=xblock.kind,
        source=xblock.source,
        value=xblock.value,
        path=xblock.path,
        description=xblock.description,
    )


#
# Change tracking
#


class ModuleMutationType(enum.Enum):
    CREATE_FILE = "CREATE_FILE"
    UPDATE_FILE = "UPDATE_FILE"
    DELETE_FILE = "DELETE_FILE"
    CREATE_STATEMENT = "CREATE_STATEMENT"
    UPDATE_STATEMENT = "UPDATE_STATEMENT"
    DELETE_STATEMENT = "DELETE_STATEMENT"


@dataclass(repr=False)
class ModuleMutation:
    module_id: UUID
    type: ModuleMutationType
    file: Optional[FileData]
    statement: Optional[StatementData]


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
