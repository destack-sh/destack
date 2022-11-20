from __future__ import annotations

from typing import Optional

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


@gql.django.type(models.ProjectVersion)
class ProjectVersion(gql.Node):
    project: Project
    name: auto
    description: auto
    parents: list[ProjectVersion]
    created_at: auto
    committed_at: auto
    program: Optional[Task]
    files: list[ProjectFile]


@gql.django.type(models.ProjectFile)
class ProjectFile(gql.Node):
    project_version: ProjectVersion = gql.django.field()
    type: auto
    name: auto
    task: Task
    instruction: Instruction
    model: Model
    dataset: Dataset

    @strawberry.field
    def name_dot_type(self) -> str:
        return f"{self.name}.{self.type}"


@gql.django.type(models.Project)
class Project(gql.Node):
    name: auto
    slug: auto
    organization: Organization
    created_at: auto
    updated_at: auto
    head: ProjectVersion
    versions: list[ProjectVersion]


@gql.django.type(models.Task)
class Task(gql.Node):
    name: auto
    created_at: auto
    updated_at: auto
    parent: Optional[Task]
    children: list[Task]
    schema: auto
    template_implementation: Optional[Instruction]


@gql.django.type(models.Instruction)
class Instruction(gql.Node):
    name: auto
    created_at: auto
    updated_at: auto
    parent: Optional[Instruction]
    children: list[Instruction]
    builtin_id: auto
    code: auto
    scope: auto


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
    records: auto
