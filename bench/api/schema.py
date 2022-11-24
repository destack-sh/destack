from __future__ import annotations

import uuid
from typing import Optional, Type, Union

import strawberry
from asgiref.sync import async_to_sync
from graphql import NoSchemaIntrospectionCustomRule
from strawberry.extensions import AddValidationRules, Extension, ParserCache, QueryDepthLimiter
from strawberry_django_plus import gql
from strawberry_django_plus.directives import SchemaDirectiveExtension
from strawberry_django_plus.optimizer import DjangoOptimizerExtension

from bench import models
from bench.api.types import Organization, Project, ProjectFile, ProjectVersion, User
from bench.compiler import Compiler, CompilerOptions
from bench.executor import Executor
from bench.settings import DEBUG, TEST


@strawberry.type
class Query:
    user: Optional[User] = gql.relay.node()
    users: gql.relay.Connection[User] = gql.relay.connection()
    project: Optional[Project] = gql.relay.node()
    projectBySlug: Optional[Project] = gql.django.field(resolver=models.Project.objects.get_by_slug)
    projects: gql.relay.Connection[Project] = gql.relay.connection()
    projectVersion: Optional[ProjectVersion] = gql.relay.node()
    projectFile: Optional[ProjectFile] = gql.relay.node()
    organization: Optional[Organization] = gql.relay.node()
    organizationBySlug: Optional[Organization] = gql.django.field(
        resolver=models.Organization.objects.get_by_slug
    )
    organizations: gql.relay.Connection[Organization] = gql.relay.connection()


@strawberry.type
class Mutation:
    @strawberry.mutation
    def compile_task(self, project_version_id: uuid.UUID, task_id: uuid.UUID) -> None:
        compiler = Compiler(Executor())
        project_version = models.ProjectVersion.objects.get(id=project_version_id)
        task = models.Task.objects.get(id=task_id)
        options = CompilerOptions(optimize_task=False, optimize_instruction=False)
        async_to_sync(compiler.compile_task)(task, project_version.backends, options)

    @strawberry.mutation
    def run_program(self, project_id: uuid.UUID) -> None:
        pass


default_extensions: list[Union[Type[Extension], Extension]] = [
    DjangoOptimizerExtension,
    QueryDepthLimiter(max_depth=10),
    SchemaDirectiveExtension,
]
prod_extensions: list[Union[Type[Extension], Extension]] = [
    ParserCache(),
    AddValidationRules([NoSchemaIntrospectionCustomRule]),
]
if DEBUG or TEST:
    extensions = default_extensions
else:
    extensions = default_extensions + prod_extensions

schema = strawberry.Schema(
    Query,
    Mutation,
    extensions=extensions,
)
