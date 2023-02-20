import typing
from typing import Optional, Union

import strawberry
from django.contrib.auth.models import AnonymousUser
from graphql import NoSchemaIntrospectionCustomRule
from strawberry.extensions import AddValidationRules, Extension, ParserCache, QueryDepthLimiter
from strawberry.types import Info
from strawberry_django_plus import gql
from strawberry_django_plus.directives import SchemaDirectiveExtension
from strawberry_django_plus.optimizer import DjangoOptimizerExtension

from bench import models
from bench.api.execution import ExecutionQuery
from bench.api.organization import Organization
from bench.api.project import File, FileMutation, Project, ProjectVersion, ProjectVersionMutation
from bench.api.runtime import ModuleRuntimeMutation, ModuleRuntimeSubscription
from bench.api.statement import StatementMutation, SymbolMutation, Type
from bench.api.user import User
from bench.settings import DEBUG, TEST

PyType = typing.Type


async def get_me(self, info: Info) -> Optional[User]:
    # unwrap because we need the actual object but channels.auth gives us a UserLazyObject
    user = info.context.request.scope["user"]._wrapped
    if isinstance(user, AnonymousUser):
        return None
    return user


@strawberry.type
class Query(ExecutionQuery):
    me = gql.django.field(resolver=get_me)
    user: Optional[User] = gql.relay.node()
    users: gql.relay.Connection[User] = gql.relay.connection()
    organization: Optional[Organization] = gql.relay.node()
    organizationBySlug: Optional[Organization] = gql.django.field(
        resolver=models.Organization.objects.get_by_slug
    )
    project: Optional[Project] = gql.relay.node()
    projectBySlug: Optional[Project] = gql.django.field(resolver=models.Project.objects.get_by_slug)
    projects: gql.relay.Connection[Project] = gql.relay.connection()
    projectVersion: Optional[ProjectVersion] = gql.relay.node()
    file: Optional[File] = gql.relay.node()


@strawberry.type
class Mutation(
    StatementMutation,
    SymbolMutation,
    FileMutation,
    ProjectVersionMutation,
    ModuleRuntimeMutation,
):
    pass


@strawberry.type
class Subscription(ModuleRuntimeSubscription):
    pass


default_extensions: list[Union[PyType[Extension], Extension]] = [
    DjangoOptimizerExtension,
    QueryDepthLimiter(max_depth=10),
    SchemaDirectiveExtension,
]
prod_extensions: list[Union[PyType[Extension], Extension]] = [
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
    Subscription,
    extensions=extensions,
    # add interface implementation types explicitly
    types=[Type],
)
