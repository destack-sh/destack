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
from bench.compiler import Compiler
from bench.executor import Executor
from bench.settings import DEBUG, TEST


@strawberry.type
class Query:
    user: Optional[types.User] = gql.relay.node()
    users: gql.relay.Connection[types.User] = gql.relay.connection()
    project: Optional[types.Project] = gql.relay.node()
    projectBySlug: Optional[types.Project] = gql.django.field(
        resolver=models.Project.objects.get_by_slug
    )
    projects: gql.relay.Connection[types.Project] = gql.relay.connection()
    projectVersion: Optional[types.ProjectVersion] = gql.relay.node()
    file: Optional[types.File] = gql.relay.node()
    organization: Optional[types.Organization] = gql.relay.node()
    organizationBySlug: Optional[types.Organization] = gql.django.field(
        resolver=models.Organization.objects.get_by_slug
    )
    organizations: gql.relay.Connection[types.Organization] = gql.relay.connection()


@strawberry.input
class CompileInput:
    compilation_id: GlobalID


@strawberry.type
class CompilePayload:
    compilation: types.Compilation


@strawberry.type
class Mutation:
    @strawberry.mutation
    def compile(self, input: CompileInput) -> CompilePayload:
        compilation = (
            models.Compilation.objects.all()
            .select_related("project_version", "task", "target_task", "target_code")
            .get(id=input.compilation_id.node_id)
        )
        executor = Executor()
        compiler = Compiler(executor)
        async_to_sync(compiler.compile)(compilation)
        return CompilePayload(compilation=compilation)


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
