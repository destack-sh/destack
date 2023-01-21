"""
Server-side mapper to translate between language and database models.

'Write' direction is language -> database, 'read' is database -> wire.
"""

from __future__ import annotations

import typing
from itertools import chain, groupby
from uuid import UUID

from django.db import transaction

from bench import language, models
from bench.language import wire
from bench.language.parse import index_module
from bench.language.type import StatementPath, StatementType, SymbolType
from bench.language.wire import render_symbol_type_node
from bench.models.project import Project, ProjectVersion


def lookup_in_db_module(
    requirement: language.Requirement, path: StatementPath
) -> language.Statement:
    version = lookup_module(requirement.name, requirement.version)
    if version is None:
        raise ValueError(f"could not find module {requirement}")

    # TODO @Performance: cache indexed module for lookup by version
    wire_module: wire.ModuleData = read_module(version, path)
    module = wire.wmap_module(wire_module)
    idx = index_module(module)
    return idx.statements_by_path.get(path)


def lookup_module(name: str, version: str) -> typing.Optional[ProjectVersion]:
    # requirement names are organization.library
    organization_slug, library_slug = name.split(".")
    try:
        library = Project.objects.get_by_slug(organization_slug, library_slug)
    except Project.DoesNotExist:
        return None
    if version == "latest":
        return library.head_
    else:
        return ProjectVersion.objects.filter(project=library, name=version).first()


@transaction.atomic(savepoint=False)  # read-only
def read_module(project_v: ProjectVersion, path: StatementPath | None = None) -> wire.ModuleData:
    """Reads the DB module to satisfy the given path. Currently, reads the entire module (ignoring path)."""
    module_name = f"{project_v.project.organization.slug}.{project_v.project.slug}"
    wire_module = wire.ModuleData(id=project_v.id, name=module_name, files=[])
    wire_files: dict[UUID, wire.FileData] = {}
    wire_statements: dict[UUID, wire.StatementData] = {}

    # map files
    for file in project_v.files.all():
        wire_file = wire.FileData(
            module_id=wire_module.id, id=file.id, path=file.path, statements=[]
        )
        wire_files[file.id] = wire_file
        wire_module.files.append(wire_file)

    # map statements
    statements = list(project_v.statements.all())
    for statement in statements:
        wire_statement = rmap_statement(statement, file=wire_files[statement.file_id])
        wire_statements[statement.id] = wire_statement
        wire_files[statement.file_id].statements.append(wire_statement)

    return wire_module


@transaction.atomic
def write_module(
    files: list[wire.FileData], project_version: models.ProjectVersion
) -> list[models.File]:
    """Write the wire files (and their contents) as models to the database."""
    wire_statements: dict[UUID, wire.StatementData] = {}
    model_files: dict[UUID, models.File] = {}
    model_statements: dict[UUID, models.Statement] = {}
    model_contents_relations: list[typing.Any] = []

    # create files
    for file_data in files:
        # remove extension from file path (assumed to be .instruct, but ignored/not stored)
        path = file_data.path
        if "." in path:
            path = file_data.path.rsplit(".", 1)[0]
        model_files[file_data.id] = project_version.create_file_from_path(path, id=file_data.id)

    # map statements
    for stmt_data in chain.from_iterable(file.statements for file in files):
        wire_statements[stmt_data.id] = stmt_data
        model_statement = models.Statement(
            id=stmt_data.id,
            project_version=project_version,
            file=model_files[stmt_data.file_id],
            parent=None,
            index=stmt_data.index,
            type=stmt_data.type,
            modifier=stmt_data.modifier,
            name=stmt_data.name,
            text=stmt_data.text,
            symbol_type=stmt_data.symbol_type,
            reference=None,
        )
        model_statements[stmt_data.id] = model_statement

        if stmt_data.type == StatementType.DEFINITION:
            new_relations = wmap_symbol(model_statement, stmt_data)
            model_contents_relations.extend(new_relations)

    # create statements
    models.Statement.objects.bulk_create(model_statements.values())
    # and their relations
    for relation_cls, relations in groupby(model_contents_relations, key=type):
        relation_cls.objects.bulk_create(relations)

    # map references (incl. parent)
    for wire_statement in wire_statements.values():
        model_statement = model_statements[wire_statement.id]
        model_statement.parent_id = wire_statement.parent_id
        model_statement.reference_id = wire_statement.reference_id
    models.Statement.objects.bulk_update(model_statements.values(), ["parent", "reference"])

    return list(model_files.values())


def rmap_statement(statement: models.Statement, file: wire.FileData) -> wire.StatementData:
    """Reads a database statement into a wire statement."""
    # map reference into wire-able reference (convert module-external ref to statement path)
    if statement.reference is not None:
        if statement.reference.project_version_id == statement.project_version_id:
            reference = statement.reference_id
        else:
            # :StatementReferencePath
            # create statement path as import path
            module_name = statement.reference.project_version.project.path
            import_source = f"{module_name}.{statement.reference.file.path}"
            reference = StatementPath(import_source, statement.reference.name)
    else:
        reference = None

    data = wire.StatementData(
        id=statement.id,
        module_id=file.module_id,
        file_id=file.id,
        revision=statement.revision,
        parent_id=statement.parent.id if statement.parent else None,
        index=statement.index,
        type=statement.type,
        modifier=statement.modifier,
        name=statement.name,
        text=statement.text,
        symbol_type=statement.symbol_type,
        reference=reference,
        reference_module=statement.reference_project_version_id,
    )
    if statement.type == StatementType.DEFINITION:
        rmap_symbol(statement, data)
    return data


def rmap_symbol(statement: models.Statement, data: wire.StatementData) -> None:
    """Reads a database statement's symbol into a wire statement."""
    data.description = statement.description
    data.lang = statement.lang
    data.code = statement.code
    data.code_builtin_id = statement.code_builtin_id
    data.provider = statement.provider
    data.external_name = statement.external_name
    data.type_node = statement.btl
    if statement.symbol_type == SymbolType.DATASET:
        data.records = list(statement.records.all().values_list("data", flat=True))
    elif statement.symbol_type == SymbolType.COMPILATION:
        data.mappings = [
            models.SourceMapping(
                compilation=statement,
                source_id=m.source_id,
                source_path=m.source_path,
                source_revision=m.source_revision,
                target_id=m.target_id,
                target_path=m.target_path,
                target_revision=m.target_revision,
            )
            for m in statement.mappings.all()
        ]
    elif statement.symbol_type == SymbolType.REQUIREMENT:
        data.reference_module = wire.ModuleReference(
            name=statement.reference_project_version.project.path,
            version=statement.reference_project_version.name,
            id=statement.reference_project_version_id,
        )


def wmap_symbol(statement: models.Statement, data: wire.StatementData) -> list[typing.Any]:
    """Writes a wire statement's symbol into a database statement."""
    statement.description = data.description
    statement.lang = data.lang
    statement.code = data.code
    statement.code_builtin_id = data.code_builtin_id
    statement.provider = data.provider
    statement.external_name = data.external_name
    if isinstance(data.type_node, language.TypeNode):
        # render type node to string
        statement.btl = render_symbol_type_node(data.symbol_type, data.type_node)
    else:
        statement.btl = data.type_node

    # copy relational data
    if data.records:
        model_records = [
            models.DatasetRecord(dataset=statement, index=i, data=data)
            for i, data in enumerate(data.records)
        ]
        return model_records
    elif data.mappings:
        mappings = [
            models.SourceMapping(
                source_id=m.source_id,
                source_path=m.source_path,
                source_revision=m.source_revision,
                target_id=m.target_id,
                target_path=m.target_path,
                target_revision=m.target_revision,
            )
            for m in data.mappings
        ]
        return mappings
    elif data.reference_module:
        if isinstance(data.reference_module, UUID):
            statement.reference_project_version_id = data.reference_module
        else:  # lookup by (name, version)
            statement.reference_project_version = lookup_module(
                data.reference_module.name, data.reference_module.version
            )

    return []
