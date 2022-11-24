from __future__ import annotations

from typing import Optional, Union

import strawberry
from strawberry import auto
from strawberry_django_plus import gql

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
    versions: list[ProjectVersion]


@gql.django.type(models.ProjectVersion)
class ProjectVersion(gql.Node):
    project: Project
    name: auto
    description: auto
    parents: list[ProjectVersion]
    created_at: auto
    committed_at: auto
    program: Optional[Task]
    files: list[File]
    references: list[ProjectVersion]


@gql.django.type(models.File)
class File(gql.Node):
    project_version: ProjectVersion = gql.django.field()
    name: auto
    is_folder: auto
    parent: Optional[File]  # containing folder
    files: list[File]  # if folder
    definitions: list[SymbolDefinition]  # if file


@gql.django.type(models.Symbol)
class Symbol(gql.Node):
    project: Project
    type: auto


@gql.django.type(models.SymbolDefinition)
class SymbolDefinition(gql.Node):
    symbol: Symbol
    project_version: ProjectVersion
    name: auto
    type: auto
    file: File
    index: auto
    created_at: auto
    updated_at: auto
    content: Union[Task, Instruction, Model, Dataset, DatasetView]

    @strawberry.field
    def name_dot_type(self) -> str:
        return f"{self.name}.{self.type}"


@gql.django.type(models.Task)
class Task(gql.Node):
    name: auto
    created_at: auto
    updated_at: auto
    parent: Optional[Task]
    index: auto
    children: list[Task]
    schema: auto
    expectations: list[Expectation]
    template_implementation: Optional[Instruction]
    implementations: list[Symbol]


@gql.django.type(models.Expectation)
class Expectation(gql.Node):
    task: Task
    index: auto
    created_at: auto
    updated_at: auto
    description: auto
    instructions: list[Symbol]
    examples_datasets: list[Symbol]


@gql.django.type(models.Instruction)
class Instruction(gql.Node):
    name: auto
    created_at: auto
    updated_at: auto
    parent: Optional[Instruction]
    children: list[Instruction]
    index: auto
    task: Optional[Symbol]
    scope: auto
    builtin_id: auto
    code: auto


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
    instruction_bound: Instruction
    instruction_free: Instruction
    name: auto
    created_at: auto
    updated_at: auto
    type: auto
    value_reference: auto
    value: auto


@gql.django.type(models.Model)
class Model(gql.Node):
    name: auto
    created_at: auto
    updated_at: auto
    baseline: Optional[Model]
    provider: auto


@gql.django.type(models.Dataset)
class Dataset(gql.Node):
    name: auto
    created_at: auto
    schema: auto
    length: auto
    records: list[DatasetRecord]


@gql.django.type(models.DatasetRecord)
class DatasetRecord(gql.Node):
    index: auto
    data: auto


@gql.django.type(models.DatasetView)
class DatasetView(gql.Node):
    name: auto
    created_at: auto
    dataset: Dataset
