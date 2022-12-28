from __future__ import annotations

import typing
from uuid import UUID

from django.db import transaction

from bench import language, models
from bench.models.project import FileType
from bench.models.symbol import SYMBOL_TYPE_TO_FIELD


class Resolver:
    """
    Server-side resolver to remotely read and write files and statements.
    Keeps track of revisions used for automatic caching and re-computation.
    """

    def read_statement(self, statement: models.Statement) -> language.Statement:
        raise NotImplementedError


@transaction.atomic
def write(files: list[language.File], project_version: models.ProjectVersion) -> list[models.File]:
    model_files: dict[UUID, models.File] = {}
    model_statements: dict[UUID, models.Statement] = {}
    model_contents: dict[UUID, typing.Any] = {}

    # create files
    for file in files:
        file_type = FileType(file.extension)
        model_file = project_version.create_file_from_path(file.path_without_extension, file_type)
        model_files[file.id] = model_file

    # create statements
    for file in files:
        model_file = model_files[file.id]
        for statement in file.statements:
            if statement.content is not None:
                model_content = map_symbol(statement.content)
                model_contents[statement.content.id] = model_content
                kwargs = {SYMBOL_TYPE_TO_FIELD[statement.content.type]: model_content}
            elif statement.requirement is not None:
                model_requirement = map_requirement(statement.requirement)
                model_contents[statement.requirement.id] = model_requirement
                kwargs = dict(requirement=model_requirement)
            elif statement.compilation is not None:
                model_compilation = map_compilation(statement.compilation)
                model_contents[statement.compilation.id] = model_compilation
                kwargs = dict(compilation=model_compilation)
            elif statement.runconfig is not None:
                model_runconfig = map_runconfig(statement.runconfig)
                model_contents[statement.runconfig.id] = model_runconfig
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
                value=statement.value,
                symbol_type=statement.symbol_type,
                reference=None,
                **kwargs,
            )
            model_statements[statement.id] = model_statement

    # create statements contents
    for content in model_contents.values():
        content.save()

    # bulk create statements
    models.Statement.objects.bulk_create(model_statements.values())

    # set references to other statements
    for statement in model_statements.values():
        if statement.parent is not None:
            statement.parent = model_statements.get(statement.parent.id)
        if statement.reference is not None and isinstance(statement.reference, language.Statement):
            statement.reference = model_statements.get(statement.reference.id)

    return list(model_files.values())


def map_symbol(content: language.SymbolContent) -> models.SymbolContent:
    if isinstance(content, language.Dataset):
        raise NotImplementedError
    raise NotImplementedError


def map_requirement(requirement: language.Requirement) -> models.Requirement:
    raise NotImplementedError


def map_compilation(compilation: language.Compilation) -> models.Compilation:
    return models.Compilation(id=compilation.id)


def map_runconfig(runconfig: language.RunConfiguration) -> models.RunConfiguration:
    return models.RunConfiguration(id=runconfig.id)
