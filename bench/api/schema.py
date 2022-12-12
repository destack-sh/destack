from __future__ import annotations

from typing import Optional, Type, Union

import strawberry
from graphql import NoSchemaIntrospectionCustomRule
from strawberry.extensions import AddValidationRules, Extension, ParserCache, QueryDepthLimiter
from strawberry_django_plus import gql
from strawberry_django_plus.directives import SchemaDirectiveExtension
from strawberry_django_plus.optimizer import DjangoOptimizerExtension

from bench import models
from bench.api import types
from bench.api.code import CodeMutation
from bench.api.compilation import CompilationMutation
from bench.api.file import FileMutation
from bench.api.symbol import SymbolMutation
from bench.api.version import ProjectVersionMutation
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
    symbol: Optional[types.Symbol] = gql.relay.node()
    organization: Optional[types.Organization] = gql.relay.node()
    organizationBySlug: Optional[types.Organization] = gql.django.field(
        resolver=models.Organization.objects.get_by_slug
    )
    organizations: gql.relay.Connection[types.Organization] = gql.relay.connection()


@strawberry.type
class Mutation(
    SymbolMutation, FileMutation, CodeMutation, CompilationMutation, ProjectVersionMutation
):
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
