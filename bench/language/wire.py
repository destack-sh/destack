import enum
import typing
from dataclasses import dataclass
from typing import Optional, Union
from uuid import UUID

from bench import language
from bench.language import ErrorType
from bench.language.parse import (
    parse_type_node,
    parse_type_node_func,
    parse_type_node_struct_inline,
    parser_from_string,
)
from bench.language.reconstruct import (
    render_type_node,
    render_type_node_func,
    render_type_node_struct,
)
from bench.language.type import (
    LiteralValue,
    SourceMapping,
    StatementModifier,
    StatementPath,
    StatementType,
    SymbolType,
    TypeNode,
    TypeTag,
)

#
# Stable, concise and flat language data structures for transit and storage.
#


@dataclass(repr=False)
class ModuleData:
    id: UUID
    name: str
    files: list["FileData"]

    def __str__(self):
        return f"{self.name}@{self.id} ({len(self.files)} files)"

    def __repr__(self):
        return f"<Module {str(self)}>"


@dataclass(repr=False)
class FileData:
    id: UUID
    module_id: UUID
    path: str
    statements: list["StatementData"]

    def __str__(self):
        return f"{self.module_id}/{self.path}"

    def __repr__(self):
        return f"<File {str(self)}>"


ModuleReference = typing.NamedTuple(
    "ModuleReference", [("name", str), ("version", str), ("id", Optional[UUID])]
)


@dataclass(repr=False)
class StatementData:
    id: UUID
    module_id: UUID
    file_id: UUID
    index: int
    revision: int
    type: StatementType
    modifier: Optional[StatementModifier]
    name: Optional[str]
    parent_id: Optional[UUID]
    reference: Union[None, StatementPath, UUID]
    text: Optional[str]
    symbol_type: Optional[SymbolType]
    # symbol contents
    type_node: Union[None, str, TypeNode] = None
    description: Optional[str] = None
    lang: Optional[str] = None
    code: Optional[str] = None
    code_builtin_id: Optional[str] = None
    provider: Optional[str] = None
    external_name: Optional[str] = None
    records: Optional[list[dict]] = None
    mappings: Optional[list[SourceMapping]] = None
    value: LiteralValue = None
    reference_module: Optional[ModuleReference] = None

    @property
    def reference_id(self) -> Optional[UUID]:
        return self.reference if isinstance(self.reference, UUID) else None

    def __str__(self):
        loc = str(self.file_id) + ":" + str(self.index)
        symbol_type_str = self.symbol_type.name if self.symbol_type else ""
        ref_str = f"ref={self.reference}" if self.reference else ""
        ref_module_str = f"ref_module={self.reference_module}" if self.reference_module else ""
        content_str = ", ".join((s for s in (ref_str, ref_module_str) if s))
        return f"{loc}: {self.type.name} {symbol_type_str} {self.name} ({content_str})"

    def __repr__(self):
        return f"<Statement {str(self)}>"


@dataclass(repr=False)
class ErrorData:
    type: ErrorType
    statement_id: Optional[UUID]
    message: str
    verbose_message: Optional[str]


def rmap_error(error: language.Error) -> ErrorData:
    return ErrorData(
        type=error.type,
        statement_id=error.statement.id if error.statement is not None else None,
        message=error.message,
        verbose_message=error.verbose_message,
    )


def rmap_module(module: language.Module) -> ModuleData:
    return ModuleData(
        id=module.id,
        name=module.name,
        files=[rmap_file(file) for file in module.files],
    )


def wmap_module(data: ModuleData) -> language.Module:
    module = language.Module(id=data.id, name=data.name)
    module.files = [wmap_file(file, module) for file in data.files]

    # restore statement references
    statements = {statement.id: statement for file in module.files for statement in file.statements}
    for data_file, file in zip(data.files, module.files):
        for data_statement, statement in zip(data_file.statements, file.statements):
            if data_statement.parent_id is not None:
                statement.parent = statements[data_statement.parent_id]
            if isinstance(data_statement.reference, UUID):
                # errors if it was a module-external reference (that's not a statement path)
                statement.reference = statements[data_statement.reference_id]

    return module


def rmap_file(file: language.File) -> FileData:
    return FileData(
        id=file.id,
        module_id=file.module.id,
        path=file.path,
        statements=[rmap_statement(statement) for statement in file.statements],
    )


def wmap_file(data: FileData, module: language.Module) -> language.File:
    file = language.File(
        id=data.id,
        module=module,
        path=data.path,
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
        revision=0,
        id=statement.id,
        index=statement.index,
        parent_id=statement.parent_id,
        type=statement.type,
        modifier=statement.modifier,
        reference=reference,
        name=statement.name,
        text=statement.text,
        symbol_type=statement.symbol_type,
    )
    if statement.content is not None:
        rmap_symbol(statement.content, data)
    return data


def rmap_symbol(content: language.SymbolContent, data: StatementData) -> None:
    """Maps a language symbol's _contents_ (excl. refs) to a wire statement."""
    if isinstance(content, language.Type):
        data.description = content.description
        data.type_node = content.type_node
    elif isinstance(content, language.Task):
        data.description = content.description
        data.type_node = content.type_node
    elif isinstance(content, language.Expectation):
        data.description = content.description
    elif isinstance(content, language.Code):
        data.description = content.description
        data.lang = content.language
        data.code = content.code
        data.code_builtin_id = content.builtin_id
        data.type_node = content.type_node
    elif isinstance(content, language.Model):
        data.provider = content.provider
        data.external_name = content.external_name
    elif isinstance(content, language.Value):
        data.description = content.description
        data.value = content.value
    elif isinstance(content, language.Capability):
        data.description = content.description
    elif isinstance(content, language.Dataset):
        data.lang = content.language
        data.description = content.description
        data.records = content.records
        data.type_node = content.type_node
    elif isinstance(content, language.Compilation):
        data.mappings = content.source_mappings
    elif isinstance(content, language.Requirement):
        if content.name and content.version:
            data.reference_module = ModuleReference(content.name, content.version, id=None)
    elif isinstance(content, language.Runconfig):
        pass
    else:
        raise ValueError(f"unexpected symbol type {content}")


def wmap_statement(data: StatementData, file: language.File) -> language.Statement:
    """Maps a wire statement's _contents_ (excl. refs) to a language statement."""
    reference = data.reference if isinstance(data.reference, StatementPath) else None
    statement = language.Statement(
        id=data.id,
        file=file,
        parent=None,  # must be restored later
        reference=reference,  # also restored later if it was an id
        index=data.index,
        type=data.type,
        modifier=data.modifier,
        name=data.name,
        text=data.text,
        symbol_type=data.symbol_type,
    )
    if statement.type == StatementType.DEFINITION:
        statement.content = wmap_symbol(data, statement)
    return statement


def wmap_symbol(data: StatementData, statement: language.Statement) -> language.SymbolContent:
    """Maps a wire statement's symbol contents to a language symbol."""
    if data.symbol_type == SymbolType.TYPE:
        return language.Type(
            definition=statement, description=data.description, type_node=data.type_node
        )
    elif data.symbol_type == SymbolType.TASK:
        return language.Task(
            definition=statement, type_node=data.type_node, description=data.description
        )
    elif data.symbol_type == SymbolType.EXPECTATION:
        return language.Expectation(definition=statement, description=data.description)
    elif data.symbol_type == SymbolType.CODE:
        return language.Code(
            definition=statement,
            description=data.description,
            language=data.lang,
            code=data.code,
            type_node=data.type_node,
            builtin_id=data.code_builtin_id,
        )
    elif data.symbol_type == SymbolType.MODEL:
        return language.Model(
            definition=statement,
            provider=data.provider,
            external_name=data.external_name,
            settings=None,
        )
    elif data.symbol_type == SymbolType.VALUE:
        return language.Value(definition=statement, description=data.description, value=data.value)
    elif data.symbol_type == SymbolType.CAPABILITY:
        return language.Capability(definition=statement, description=data.description)
    elif data.symbol_type == SymbolType.DATASET:
        return language.Dataset(
            definition=statement,
            description=data.description,
            language=data.lang,
            type_node=data.type_node,
            records=data.records,
        )
    elif data.symbol_type == SymbolType.COMPILATION:
        return language.Compilation(definition=statement, source_mappings=data.mappings)
    elif data.symbol_type == SymbolType.REQUIREMENT:
        return language.Requirement(
            definition=statement,
            name=data.reference_module.name if data.reference_module else None,
            version=data.reference_module.version if data.reference_module else None,
        )
    elif data.symbol_type == SymbolType.RUNCONFIG:
        return language.Runconfig(definition=statement)
    else:
        raise ValueError(
            f"unexpected symbol type {statement.symbol_type} for statement {statement}"
        )


def render_symbol_type_node(symbol_type: language.SymbolType, type_node: language.TypeNode) -> str:
    """Renders a type node into a recoverable string."""
    if symbol_type in (SymbolType.TASK, SymbolType.CODE):
        return render_type_node_func(type_node)
    elif symbol_type == SymbolType.DATASET:
        return f"({render_type_node_struct(type_node, seperator=', ')})"
    elif symbol_type == SymbolType.TYPE:
        if type_node.type == TypeTag.STRUCT:
            # unlike the others, this node is not rendered as it appears in Bench
            # because we need to distinguish struct defs from inline redefs
            # (and Bench structs don't have any special characters and may be empty)
            return f"({render_type_node_struct(type_node, seperator=', ')})"
        else:
            return render_type_node(type_node)
    else:
        raise ValueError(f"unexpected symbol type {symbol_type}")


def parse_symbol_type_node(symbol_type: language.SymbolType, type_node: str) -> language.TypeNode:
    """Parses a type node from a recoverable string (serialized like above)."""
    btl_parser = parser_from_string(type_node)
    if symbol_type in (SymbolType.TASK, SymbolType.CODE):
        parsed = parse_type_node_func(btl_parser, name=None)
    elif symbol_type == SymbolType.DATASET:
        btl_parser.eat_bracket("(")
        parsed = parse_type_node_struct_inline(btl_parser, name=None)
        btl_parser.eat_bracket(")")
    elif symbol_type == SymbolType.TYPE:
        # hacky way to determine whether it's an inline redef or struct def
        if type_node.startswith("("):
            btl_parser.eat_bracket("(")
            parsed = parse_type_node_struct_inline(btl_parser, name=None)
            btl_parser.eat_bracket(")")
        else:
            parsed = parse_type_node(btl_parser, name=None)
    else:
        raise ValueError(f"unexpected symbol type {symbol_type}")
    btl_parser.eat_eos()  # must be full match
    return parsed


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
