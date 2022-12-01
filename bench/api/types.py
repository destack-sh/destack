from __future__ import annotations

from typing import Optional

from strawberry import auto
from strawberry_django_plus import gql

import bench.models.symbol
from bench import models


@gql.django.type(models.User)
class User(gql.relay.Node):
    username: auto
    first_name: auto
    last_name: auto
    email: auto
    created_at: auto
    updated_at: auto
    organizations: list[Organization]


@gql.django.type(models.Organization)
class Organization(gql.relay.Node):
    name: auto
    slug: auto
    created_at: auto
    updated_at: auto
    projects: list[Project]
    members: list[User]


@gql.django.type(models.Project)
class Project(gql.Node):
    name: auto
    slug: auto
    organization: Organization
    created_at: auto
    updated_at: auto
    head: ProjectVersion
    versions: list[ProjectVersion]  # TODO @Cleanup: use relay connections


@gql.django.type(models.ProjectVersion)
class ProjectVersion(gql.Node):
    project: Project
    name: auto
    description: auto
    parents: list[ProjectVersion]
    children: list[ProjectVersion]
    created_at: auto
    committed_at: auto
    program: Optional[Task]
    files: list[File]
    definitions: list[SymbolDefinition]


@gql.django.type(models.File)
class File(gql.Node):
    project_version: ProjectVersion
    name: auto
    created_at: auto
    updated_at: auto
    is_folder: auto
    parent: Optional[File]  # containing folder
    files: list[File]  # if folder
    definitions: list[SymbolDefinition]  # if file


@gql.django.type(bench.models.symbol.SymbolDefinition)
class SymbolDefinition(gql.Node):
    project_version: ProjectVersion
    name: auto
    type: auto
    name_dot_type: auto
    file: File
    parent: Optional[SymbolDefinition]
    children: list[SymbolDefinition]
    index: auto
    created_at: auto
    updated_at: auto
    committed_in: Optional[ProjectVersion]
    committed: auto
    content: SymbolContent


@gql.django.interface(models.SymbolContent)
class SymbolContent(gql.Node):
    pass
    # TODO @Cleanup: fix definition in SymbolContent interface
    #  (should work since it's a 1:1 but doesn't)
    # definition: SymbolDefinition


@gql.django.type(models.Task)
class Task(SymbolContent):
    schema: auto
    expectations: list[Expectation]
    template_implementation: Optional[SymbolDefinition]
    compilations: list[Compilation]


@gql.django.type(models.Compilation)
class Compilation(gql.Node):
    created_at: auto
    updated_at: auto
    task: Task
    name: auto
    backends: list[SymbolDefinition]
    output_task: Optional[SymbolDefinition]
    output_instruction: Optional[SymbolDefinition]


@gql.django.type(models.Expectation)
class Expectation(SymbolContent):
    task: Task
    description: auto
    statements: list[SymbolDefinition]


@gql.django.type(models.Instruction)
class Instruction(SymbolContent):
    task: Optional[Task]
    scope: auto
    builtin_id: auto
    code: auto
    parameters: list[InstructionParameter]
    arguments: list[InstructionArgument]


@gql.django.type(models.InstructionParameter)
class InstructionParameter(gql.Node):
    instruction: Instruction
    name: auto
    created_at: auto
    updated_at: auto
    type: auto
    schema: auto


@gql.django.type(models.InstructionArgument)
class InstructionArgument(gql.Node):
    instruction: Instruction
    instruction_free: Instruction
    name: auto
    created_at: auto
    updated_at: auto
    type: auto
    reference: Optional[SymbolDefinition]
    value: auto


@gql.django.type(models.Model)
class Model(SymbolContent):
    baseline: Optional[Model]
    provider: auto


@gql.django.type(models.Dataset)
class Dataset(SymbolContent):
    schema: auto
    length: auto
    records: list[DatasetRecord]


@gql.django.type(models.DatasetRecord)
class DatasetRecord(gql.Node):
    index: auto
    data: auto


@gql.django.type(models.DatasetView)
class DatasetView(SymbolContent):
    dataset: Dataset
