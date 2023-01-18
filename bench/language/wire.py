from dataclasses import dataclass
from typing import Optional, Union
from uuid import UUID

from bench import language
from bench.language import ErrorType
from bench.language.type import (
    LiteralValue,
    SourceMapping,
    StatementModifier,
    StatementPath,
    StatementType,
    SymbolType,
    TypeNode,
)

#
# Stable, optimized and flat language data structures for transit and storage.
#


@dataclass(repr=False)
class ModuleData:
    id: UUID
    name: str
    files: list["FileData"]


@dataclass(repr=False)
class FileData:
    id: UUID
    module_id: UUID
    path: str
    statements: list["StatementData"]


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
    reference_module: Union[None, UUID, tuple[str, str]] = None

    @property
    def reference_id(self) -> Optional[UUID]:
        return self.reference if isinstance(self.reference, UUID) else None


@dataclass(repr=False)
class ErrorData:
    type: ErrorType
    statement_id: Optional[UUID]
    message: str


def rmap_module(module: language.Module) -> ModuleData:
    return ModuleData(
        id=module.id,
        name=module.name,
        files=[rmap_file(file) for file in module.files],
    )


def wmap_module(data: ModuleData) -> language.Module:
    module = language.Module(id=data.id, name=data.name)
    module.files = [wmap_file(file, module) for file in data.files]
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
    file.statement = [wmap_statement(statement, file) for statement in data.statements]
    return file


def rmap_statement(statement: language.Statement) -> StatementData:
    data = StatementData(
        module_id=statement.file.module.id,
        file_id=statement.file.id,
        revision=0,
        id=statement.id,
        index=statement.index,
        parent_id=statement.parent_id,
        type=statement.type,
        modifier=statement.modifier,
        reference=statement.reference,
        name=statement.name,
        text=statement.text,
        symbol_type=statement.symbol_type,
    )
    rmap_symbol(statement.content, data)
    return data


def rmap_symbol(content: language.SymbolContent, data: StatementData) -> None:
    if isinstance(content, language.Type):
        data.description = content.description
        data.type_node = content.type_node
    elif isinstance(content, language.Task):
        data.description = content.description
    elif isinstance(content, language.Expectation):
        data.description = content.description
    elif isinstance(content, language.Code):
        data.description = content.description
        data.lang = content.language
        data.code = content.code
        data.code_builtin_id = content.builtin_id
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
    elif isinstance(content, language.Compilation):
        data.mappings = content.source_mappings
    elif isinstance(content, language.Requirement):
        data.reference_module = (content.name, content.version)
    elif isinstance(content, language.Runconfig):
        pass
    else:
        raise ValueError(f"unexpected symbol type {content}")


def wmap_statement(data: StatementData, file: language.File) -> language.Statement:
    statement = language.Statement(
        id=data.id,
        file=file,
        parent=None,
        index=data.index,
        type=data.type,
        modifier=data.modifier,
        name=data.name,
        text=data.text,
        symbol_type=data.symbol_type,
    )
    if statement.type == StatementType.DEFINITION:
        wmap_symbol(data, statement)
    return statement


def wmap_symbol(data: StatementData, statement: language.Statement) -> language.SymbolContent:
    if data.type_node is not None:
        return language.Type(
            definition=statement, description=data.description, type_node=data.type_node
        )
    elif data.description is not None:
        return language.Task(
            definition=statement, type_node=data.type_node, description=data.description
        )
    elif data.lang is not None:
        return language.Code(
            definition=statement,
            description=data.description,
            language=data.lang,
            code=data.code,
            type_node=data.type_node,
            builtin_id=data.code_builtin_id,
        )
    elif data.provider is not None:
        return language.Model(
            definition=statement,
            provider=data.provider,
            external_name=data.external_name,
        )
    elif data.value is not None:
        return language.Value(definition=statement, description=data.description, value=data.value)
    elif data.description is not None:
        return language.Capability(definition=statement, description=data.description)
    elif data.records is not None:
        return language.Dataset(
            definition=statement,
            description=data.description,
            language=data.lang,
            type_node=data.type_node,
            records=data.records,
        )
    elif data.mappings is not None:
        return language.Compilation(definition=statement, source_mappings=data.mappings)
    elif data.reference_module is not None:
        return language.Requirement(
            definition=statement,
            name=data.reference_module[0],
            version=data.reference_module[1],
        )
    else:
        raise ValueError(
            f"unexpected symbol type {statement.symbol_type} for statement {statement}"
        )
