"""
Server-side mapper to translate between language and database models.

'Write' direction is language -> database, 'read' is database -> language.
"""

from __future__ import annotations

import typing
from itertools import chain, groupby
from uuid import UUID

from django.db import transaction

from bench import language, models
from bench.language.parse import (
    index_module,
    parse_type_node,
    parse_type_node_func,
    parse_type_node_struct_inline,
    parser_from_string,
)
from bench.language.reconstruct import render_type_node
from bench.language.type import StatementPath, StatementType, SymbolType
from bench.models.project import FileType, Project, ProjectVersion
from bench.utils.record import RecordList


def lookup_module_in_db(
    requirement: language.Requirement, path: StatementPath
) -> language.Statement:
    version = lookup_requirement(requirement)
    if version is None:
        raise ValueError(f"could not find module {requirement}")

    # TODO @Performance: cache indexed module for lookup by version
    module: language.Module = read(version, path)
    idx = index_module(module)
    return idx.statements_by_path.get(path)


@transaction.atomic(savepoint=False)  # read-only
def read(project_v: ProjectVersion, path: StatementPath) -> language.Module:
    """Reads the DB module to satisfy the given path. Currently, reads the entire module (ignoring path)."""
    lang_module = rmap_module(project_v)
    lang_files: dict[UUID, language.File] = {}
    lang_statements: dict[UUID, language.Statement] = {}

    # map files
    for file in project_v.files.all():
        lang_file = language.File(module=lang_module, id=file.id, path=file.path)
        lang_files[file.id] = lang_file
        lang_module.files.append(lang_file)

    # map statements
    for statement in project_v.statements.all():
        lang_statement = rmap_statement(statement, file=lang_files[statement.file_id])
        lang_statements[statement.id] = lang_statement
        lang_statement.file.statements.append(lang_statement)

    # map references (incl. parent)
    for statement in project_v.statements.all():
        lang_statement = lang_statements[statement.id]
        lang_statement.parent = lang_statements.get(statement.parent_id)
        lang_statement.reference = lang_statements.get(statement.reference_id)

    return lang_module


def rmap_statement(statement: models.Statement, file: language.File):
    """Reads a database statement into a language statement (without references)."""
    lang_statement = language.Statement(
        id=statement.id,
        file=file,
        parent=None,
        index=statement.index,
        type=statement.type,
        modifier=statement.modifier,
        name=statement.name,
        text=statement.text,
        symbol_type=statement.symbol_type,
        reference=None,
    )
    if statement.type == StatementType.DEFINITION:
        lang_statement.content = rmap_symbol(statement, lang_statement)
    return lang_statement


@transaction.atomic
def write(files: list[language.File], project_version: models.ProjectVersion) -> list[models.File]:
    """Write the language files (and their contents) as models to the database."""
    lang_statements: dict[UUID, language.Statement] = {}
    model_files: dict[UUID, models.File] = {}
    model_statements: dict[UUID, models.Statement] = {}
    model_contents_relations: list[typing.Any] = []

    # create files
    for file in files:
        file_type = FileType(file.extension)
        model_files[file.id] = project_version.create_file_from_path(
            file.path_without_extension, file_type, id=file.id
        )

    # map statements
    for statement in chain.from_iterable(file.statements for file in files):
        lang_statements[statement.id] = statement
        model_statement = models.Statement(
            id=statement.id,
            project_version=project_version,
            file=model_files[statement.file.id],
            parent=None,
            index=statement.index,
            type=statement.type,
            modifier=statement.modifier,
            name=statement.name,
            text=statement.text,
            symbol_type=statement.symbol_type,
            reference=None,
        )
        model_statements[statement.id] = model_statement
        if statement.content is not None:
            relations = wmap_symbol(model_statement, statement.content)
            model_contents_relations.extend(relations)

    # create statements
    models.Statement.objects.bulk_create(model_statements.values())
    # and their relations
    for relation_cls, relations in groupby(model_contents_relations, key=type):
        relation_cls.objects.bulk_create(relations)

    # map references (incl. parent)
    for lang_statement in lang_statements.values():
        model_statement = model_statements[lang_statement.id]
        model_statement.parent_id = lang_statement.parent_id
        model_statement.reference_id = lang_statement.reference_id
    models.Statement.objects.bulk_update(model_statements.values(), ["parent", "reference"])

    return list(model_files.values())


def rmap_module(project_v: ProjectVersion) -> language.Module:
    module_name = f"{project_v.project.organization.slug}.{project_v.project.slug}"
    module = language.Module(name=module_name, files=[])
    return module


def wmap_symbol(statement: models.Statement, content: language.SymbolContent) -> list[typing.Any]:
    """Maps language symbol content to database models."""
    if isinstance(content, language.Type):
        statement.description = content.description
        statement.btl = render_type_node(content.node)
        return []
    elif isinstance(content, language.Capability):
        statement.description = content.description
        return []
    elif isinstance(content, language.Task):
        statement.description = content.description
        statement.btl = render_type_node(content.func_type)
        return []
    elif isinstance(content, language.Expectation):
        statement.description = content.description
        return []
    elif isinstance(content, language.Model):
        statement.provider = content.provider
        statement.external_name = content.external_name
        statement.default_settings = content.settings
        return []
    elif isinstance(content, language.Code):
        statement.code = content.code
        statement.btl = render_type_node(content.func_type)
        return []
    elif isinstance(content, language.Dataset):
        statement.btl = render_type_node(content.element_type)
        model_records = [
            models.DatasetRecord(dataset=statement, index=i, data=data)
            for i, data in enumerate(content.records)
        ]
        return model_records
    elif isinstance(content, language.Value):
        statement.value = content.value
        return []
    elif isinstance(content, language.Compilation):
        source_mappings = [
            models.SourceMapping(
                compilation=statement,
                source_id=m.source_id,
                source_path=m.source_path,
                source_revision=m.source_revision,
                target_id=m.target_id,
                target_path=m.target_path,
                target_revision=m.target_revision,
            )
            for m in content.source_mappings
        ]
        return source_mappings
    elif isinstance(content, language.Requirement):
        statement.reference_project_version = lookup_requirement(content)
        return []
    elif isinstance(content, language.Runconfig):
        return []
    else:
        raise ValueError(f"unexpected symbol content: {content}")


def rmap_symbol(
    statement: models.Statement, definition: language.Statement
) -> language.SymbolContent:
    """Maps database symbol content to language models."""
    if statement.symbol_type == SymbolType.TYPE:
        btl_parser = parser_from_string(statement.btl)
        type_node = parse_type_node(btl_parser, name=None)
        return language.Type(definition, description=statement.description, node=type_node)
    elif statement.symbol_type == SymbolType.CAPABILITY:
        return language.Capability(definition, description=statement.description)
    elif statement.symbol_type == SymbolType.TASK:
        btl_parser = parser_from_string(statement.btl)
        func_type = parse_type_node_func(btl_parser, name=statement.name)
        return language.Task(definition, description=statement.description, func_type=func_type)
    elif statement.symbol_type == SymbolType.EXPECTATION:
        return language.Expectation(definition, description=statement.description)
    elif statement.symbol_type == SymbolType.MODEL:
        return language.Model(
            definition=definition,
            provider=statement.provider,
            external_name=statement.external_name,
            settings=statement.default_settings,
        )
    elif statement.symbol_type == SymbolType.CODE:
        btl_parser = parser_from_string(statement.btl)
        func_type = parse_type_node_func(btl_parser, name=statement.name)
        return language.Code(
            definition=definition,
            # only python is supported for now
            language="python",
            code=statement.code,
            builtin_id=statement.code_builtin_id,
            func_type=func_type,
        )
    elif statement.symbol_type == SymbolType.DATASET:
        btl_parser = parser_from_string(statement.btl)
        element_type = parse_type_node_struct_inline(btl_parser, name=statement.name)
        return language.Dataset(
            definition=definition,
            records=RecordList([record.data for record in statement.records.all()]),
            element_type=element_type,
        )
    elif statement.symbol_type == SymbolType.VALUE:
        return language.Value(definition=definition, value=statement.value)
    elif statement.symbol_type == SymbolType.COMPILATION:
        source_mappings = [
            language.SourceMapping(
                source_id=m.source.id,
                source_path=m.source_path,
                source_revision=m.source_revision,
                target_id=m.target.id,
                target_path=m.target_path,
                target_revision=m.target_revision,
            )
            for m in statement.mappings.all()
        ]
        return language.Compilation(definition=definition, source_mappings=source_mappings)
    elif statement.symbol_type == SymbolType.REQUIREMENT:
        if statement.reference_project_version:
            return language.Requirement(
                definition=definition,
                name=statement.reference_project_version.project.name,
                version=statement.reference_project_version.name,
            )
        else:
            return language.Requirement(definition=definition, name=statement.name)
    elif statement.symbol_type == SymbolType.RUNCONFIG:
        return language.Runconfig(definition=definition)
    else:
        raise ValueError(f"unexpected symbol type: {statement}")


def lookup_requirement(requirement: language.Requirement) -> typing.Optional[ProjectVersion]:
    # requirement names are organization.library
    organization_slug, library_slug = requirement.name.split(".")
    try:
        library = Project.objects.get_by_slug(organization_slug, library_slug)
    except Project.DoesNotExist:
        return None
    if requirement.version == "latest":
        return library.head_
    else:
        return ProjectVersion.objects.filter(project=library, name=requirement.version).first()
