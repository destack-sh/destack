from __future__ import annotations

from typing import Optional, Union
from uuid import UUID

import strawberry
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

    @strawberry.field()
    def version(self, version_id: UUID) -> Optional[ProjectVersion]:
        return self.versions.all().filter(id=version_id).first()


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
    is_folder: auto
    parent: Optional[File]  # containing folder
    files: list[File]  # if folder
    definitions: list[SymbolDefinition]  # if file


@gql.django.type(bench.models.symbol.Symbol)
class Symbol(gql.Node):
    project: Project
    type: auto

    @strawberry.field()
    def definition(self, project_version_id: UUID) -> Optional[SymbolDefinition]:
        return self.resolve(project_version_id)


@gql.django.type(bench.models.symbol.SymbolDefinition)
class SymbolDefinition(gql.Node):
    symbol: Symbol
    project_version: ProjectVersion
    name: auto
    type: auto
    name_dot_type: auto
    file: File
    parent: Optional[Symbol]
    children: list[Symbol]
    index: auto
    created_at: auto
    updated_at: auto
    content: Union[Task, Expectation, Instruction, Model, Dataset, DatasetView]


@gql.django.interface(models.SymbolContent)
class SymbolContent(gql.Node):
    created_at: auto
    updated_at: auto
    committed_in: Optional[ProjectVersion]
    committed: auto


@gql.django.type(models.Task)
class Task(SymbolContent):
    schema: auto
    expectations: list[Symbol]
    template_implementation: Optional[Symbol]
    implementations: list[Symbol]


@gql.django.type(models.Expectation)
class Expectation(SymbolContent):
    task: Task
    created_at: auto
    updated_at: auto
    description: auto
    statements: list[ExpectationStatement]


@gql.django.type(models.ExpectationStatement)
class ExpectationStatement(gql.Node):
    expectation: Expectation
    statement: Symbol


@gql.django.type(models.Instruction)
class Instruction(SymbolContent):
    task: Optional[Symbol]
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
    instruction_bound: Instruction
    instruction_free: Instruction
    name: auto
    created_at: auto
    updated_at: auto
    type: auto
    reference: Symbol
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
    dataset: Symbol
