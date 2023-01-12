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
from bench.language.parse import index_module
from bench.language.type import StatementPath
from bench.models.project import FileType, Project, ProjectVersion
from bench.models.symbol import CONTENT_FIELDS
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
    lang_files: dict[UUID, language.File] = {}
    lang_statements: dict[UUID, language.Statement] = {}

    module = rmap_module(project_v)
    model_statements = project_v.statements.select_related(*CONTENT_FIELDS).all()

    # map files
    for file in project_v.files.all():
        lang_file = language.File(module=module, id=file.id, path=file.path)
        lang_files[file.id] = lang_file
        module.files.append(lang_file)

    # map statements
    for statement in model_statements:
        lang_statement = language.Statement(
            id=statement.id,
            file=lang_files[statement.file_id],
            parent=None,
            index=statement.index,
            type=statement.type,
            modifier=statement.modifier,
            name=statement.name,
            text=statement.text,
            symbol_type=statement.symbol_type,
            reference=None,
        )
        lang_statements[statement.id] = lang_statement
        lang_statement.file.statements.append(lang_statement)

        if statement.content is not None:
            lang_statement.content = rmap_symbol(statement.content, lang_statement)
        elif statement.requirement is not None:
            lang_statement.requirement = rmap_requirement(statement.requirement)
        elif statement.runconfig is not None:
            lang_statement.runconfig = rmap_runconfig(statement.runconfig)
        elif statement.compilation is not None:
            lang_statement.compilation = rmap_compilation(statement.compilation)

    # wmap references (incl. parent)
    for statement in model_statements:
        lang_statement = lang_statements[statement.id]
        lang_statement.parent = lang_statements.get(statement.parent_id)
        lang_statement.reference = lang_statements.get(statement.reference_id)

    return module


@transaction.atomic
def write(files: list[language.File], project_version: models.ProjectVersion) -> list[models.File]:
    """Write the language files (and their contents) as models to the database."""
    lang_statements: dict[UUID, language.Statement] = {}
    model_files: dict[UUID, models.File] = {}
    model_statements: dict[UUID, models.Statement] = {}
    model_contents: dict[UUID, typing.Any] = {}
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
            model_content, relations = wmap_symbol(statement.content)
            model_contents[statement.id] = model_content
            model_contents_relations.extend(relations)
            model_statement.set_content(model_content)
        elif statement.requirement is not None:
            model_requirement = wmap_requirement(statement.requirement)
            model_contents[statement.id] = model_requirement
            model_statement.requirement = model_requirement
        elif statement.compilation is not None:
            model_compilation, relations = wmap_compilation(statement.compilation)
            model_contents[statement.id] = model_compilation
            model_contents_relations.extend(relations)
            model_statement.compilation = model_compilation
        elif statement.runconfig is not None:
            model_runconfig = wmap_runconfig(statement.runconfig)
            model_contents[statement.id] = model_runconfig
            model_statement.runconfig = model_runconfig

    # create statements contents
    for content_cls, contents in groupby(model_contents.values(), key=type):
        content_cls.objects.bulk_create(contents)
    # and their relations
    for relation_cls, relations in groupby(model_contents_relations, key=type):
        relation_cls.objects.bulk_create(relations)

    # create actual statements
    models.Statement.objects.bulk_create(model_statements.values())

    # map references (incl. parent)
    for lang_statement in lang_statements.values():
        model_statement = model_statements[lang_statement.id]
        model_statement.parent_id = lang_statement.parent_id
        model_statement.reference_id = lang_statement.reference_id
    models.Statement.objects.bulk_update(model_statements.values(), ["parent", "reference"])

    # update derived project version data
    # TODO @Cleanup: storing any derived semantic data in the DB may be a bad idea
    project_version.derive_dependencies()

    return list(model_files.values())


def rmap_module(project_v: ProjectVersion) -> language.Module:
    module_name = f"{project_v.project.organization.slug}.{project_v.project.slug}"
    module = language.Module(name=module_name, files=[])
    return module


def wmap_symbol(content: language.SymbolContent) -> tuple[models.SymbolContent, list[typing.Any]]:
    """Maps language symbol content to database models."""
    if isinstance(content, language.Schema):
        return models.Schema(description=content.description, element=content.element), []
    elif isinstance(content, language.Task):
        return models.Task(description=content.description), []
    elif isinstance(content, language.Expectation):
        return models.Expectation(description=content.description), []
    elif isinstance(content, language.Model):
        model = models.Model(
            provider=content.provider,
            external_name=content.external_name,
            default_settings=content.settings,
        )
        return model, []
    elif isinstance(content, language.Code):
        model_code = models.Code(code=content.code, builtin_id=content.builtin_id)
        return model_code, []
    elif isinstance(content, language.Dataset):
        model_dataset = models.Dataset()
        model_records = [
            models.DatasetRecord(dataset=model_dataset, index=i, data=data)
            for i, data in enumerate(content.records)
        ]
        return model_dataset, model_records
    elif isinstance(content, language.Value):
        return models.Value(value=content.value), []
    else:
        raise ValueError(f"unexpected symbol content: {content}")


def rmap_symbol(
    content: models.SymbolContent, definition: language.Statement
) -> language.SymbolContent:
    """Maps database symbol content to language models."""
    if isinstance(content, models.Schema):
        return language.Schema(definition, description=content.description, element=content.element)
    elif isinstance(content, models.Task):
        return language.Task(definition, description=content.description)
    elif isinstance(content, models.Expectation):
        return language.Expectation(definition, description=content.description)
    elif isinstance(content, models.Model):
        return language.Model(
            definition=definition,
            provider=content.provider,
            external_name=content.external_name,
            settings=content.default_settings,
        )
    elif isinstance(content, models.Code):
        # only python is supported for now
        return language.Code(
            definition=definition,
            language="python",
            code=content.code,
            builtin_id=content.builtin_id,
        )
    elif isinstance(content, models.Dataset):
        return language.Dataset(
            definition=definition,
            records=RecordList([record.data for record in content.records.all()]),
        )
    elif isinstance(content, models.Value):
        return language.Value(definition=definition, value=content.value)
    else:
        raise ValueError(f"unexpected symbol content: {content}")


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


def wmap_requirement(
    requirement: language.Requirement,
) -> models.Requirement:
    version = lookup_requirement(requirement)
    if version is None:
        raise ValueError(f"requirement not found: {requirement}")
    return models.Requirement(project_version=version)


def rmap_requirement(requirement: models.Requirement) -> language.Requirement:
    return language.Requirement(
        name=requirement.project_version.project.slug,
        version=requirement.project_version.name,
    )


def wmap_compilation(
    compilation: language.Compilation,
) -> tuple[models.Compilation, list[typing.Any]]:
    model_compilation = models.Compilation()
    source_mappings = [
        models.SourceMapping(
            compilation=compilation,
            source_id=m.source.id,
            source_path=m.source_path,
            source_revision=m.source_revision,
            target_id=m.target.id,
            target_path=m.target_path,
            target_revision=m.target_revision,
        )
        for m in compilation.source_mappings
    ]
    return model_compilation, source_mappings


def rmap_compilation(compilation: models.Compilation) -> language.Compilation:
    return language.Compilation(
        source_mappings=[
            language.SourceMapping(
                source=compilation.source,
                source_path=m.source_path,
                source_revision=m.source_revision,
                target=compilation.target,
                target_path=m.target_path,
                target_revision=m.target_revision,
            )
            for m in compilation.mappings.all()
        ]
    )


def wmap_runconfig(
    runconfig: language.RunConfiguration,
) -> models.RunConfiguration:
    return models.RunConfiguration()


def rmap_runconfig(runconfig: models.RunConfiguration) -> language.RunConfiguration:
    return language.RunConfiguration()
