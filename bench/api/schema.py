from __future__ import annotations

from typing import Optional, Type, Union

import strawberry
from asgiref.sync import async_to_sync
from graphql import NoSchemaIntrospectionCustomRule
from strawberry.extensions import AddValidationRules, Extension, ParserCache, QueryDepthLimiter
from strawberry_django_plus import gql
from strawberry_django_plus.directives import SchemaDirectiveExtension
from strawberry_django_plus.optimizer import DjangoOptimizerExtension
from strawberry_django_plus.relay import GlobalID

from bench import models
from bench.api import types
from bench.api.types import File, Organization, Project, ProjectVersion, User
from bench.compiler import Compiler
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
    file: Optional[File] = gql.relay.node()
    organization: Optional[Organization] = gql.relay.node()
    organizationBySlug: Optional[Organization] = gql.django.field(
        resolver=models.Organization.objects.get_by_slug
    )
    organizations: gql.relay.Connection[Organization] = gql.relay.connection()


@strawberry.type
class Mutation:
    @strawberry.mutation
    def compile(self, compilation_id: GlobalID) -> None:
        compilation = (
            models.Compilation.objects.all()
            .select_related("project_version", "task", "target_task", "target_code")
            .get(id=compilation_id.node_id)
        )
        executor = Executor()
        compiler = Compiler(executor)
        async_to_sync(compiler.compile)(compilation)


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
    # add interface implementation types explicitly
    types=[
        types.Task,
        types.Expectation,
        types.Code,
        types.Model,
        types.Dataset,
        types.DatasetView,
    ],
)
