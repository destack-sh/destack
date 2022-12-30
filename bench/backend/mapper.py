from __future__ import annotations

import typing
from uuid import UUID

from django.db import transaction

from bench import language, models
from bench.models.project import FileType, Project, ProjectVersion
from bench.models.symbol import SYMBOL_TYPE_TO_FIELD


class Mapper:
    """
    Server-side mapper to remotely read and write files and statements.
    Keeps track of revisions and follows references in read/write operations.
    """

    pass


def read_statement(statement: models.Statement) -> language.Statement:
    raise NotImplementedError


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
        model_file = project_version.create_file_from_path(
            file.path_without_extension, file_type, id=file.id
        )
        model_files[file.id] = model_file

    # create statements
    for file in files:
        model_file = model_files[file.id]
        for statement in file.statements:
            lang_statements[statement.id] = statement
            if statement.content is not None:
                model_content, relations = map_symbol(statement.content)
                model_contents[statement.id] = model_content
                model_contents_relations.extend(relations)
                kwargs = {SYMBOL_TYPE_TO_FIELD[statement.content.type]: model_content}
            elif statement.requirement is not None:
                model_requirement = map_requirement(statement.requirement)
                model_contents[statement.id] = model_requirement
                kwargs = dict(requirement=model_requirement)
            elif statement.compilation is not None:
                model_compilation, relations = map_compilation(statement.compilation)
                model_contents[statement.id] = model_compilation
                model_contents_relations.extend(relations)
                kwargs = dict(compilation=model_compilation)
            elif statement.runconfig is not None:
                model_runconfig = map_runconfig(statement.runconfig)
                model_contents[statement.id] = model_runconfig
                kwargs = dict(runconfig=model_runconfig)
            else:
                kwargs = {}

            model_statement = models.Statement(
                id=statement.id,
                project_version=project_version,
                file=model_file,
                parent=None,
                index=statement.index,
                type=statement.type,
                modifier=statement.modifier,
                name=statement.name,
                text=statement.text,
                symbol_type=statement.symbol_type,
                reference=None,
                **kwargs,
            )
            model_statements[statement.id] = model_statement

    # create statements contents
    for content in model_contents.values():
        content.save()

    # create statements relations
    for relation in model_contents_relations:
        relation.save()

    # bulk create statements
    models.Statement.objects.bulk_create(model_statements.values())

    # set references to other statements
    for lang_statement in lang_statements.values():
        model_statement = model_statements[lang_statement.id]
        if lang_statement.parent is not None:
            model_statement.parent = model_statements.get(lang_statement.parent.id)
        if lang_statement.reference is not None and isinstance(
            lang_statement.reference, language.Statement
        ):
            model_statement.reference = model_statements.get(lang_statement.reference.id)
    models.Statement.objects.bulk_update(model_statements.values(), ["parent", "reference"])

    return list(model_files.values())


def map_symbol(content: language.SymbolContent) -> tuple[models.SymbolContent, list[typing.Any]]:
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
        )
        return model, []
    elif isinstance(content, language.Code):
        model_code = models.Code(
            code=content.code,
            code_function_name=content.code_function_name,
            builtin_id=content.builtin_id,
        )
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


def map_requirement(
    requirement: language.Requirement,
) -> models.Requirement():
    # requirement names are organization.library
    organization_slug, library_slug = requirement.name.split(".")
    try:
        library = Project.objects.get_by_slug(organization_slug, library_slug)
    except Project.DoesNotExist:
        raise ValueError(f"{requirement} could not be resolved")

    if requirement.version == "latest":
        version = library.head_
    else:
        version = ProjectVersion.objects.get(project=library, name=requirement.version)

    return models.Requirement(project_version=version)


def map_compilation(
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


def map_runconfig(
    runconfig: language.RunConfiguration,
) -> tuple[models.RunConfiguration, list[typing.Any]]:
    return models.RunConfiguration(), []
