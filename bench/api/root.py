import typing
from typing import Optional, Union

import strawberry
from asgiref.sync import sync_to_async
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
from bench.api.project import (
    File,
    FileMutation,
    Project,
    ProjectMutation,
    ProjectVersion,
    ProjectVersionMutation,
)
from bench.api.runtime import ModuleRuntimeMutation, ModuleRuntimeSubscription
from bench.api.statement import StatementMutation, SymbolMutation, Type
from bench.api.user import User, UserMutation
from bench.models import OwnerSlug
from bench.settings import DEBUG, TEST

PyType = typing.Type


@sync_to_async
def get_user_or_organization_by_slug(
    self, info: Info, slug: str
) -> Optional[Union[User, Organization]]:
    slug = OwnerSlug.objects.get(slug=slug)
    return slug.owner


def get_me(self, info: Info) -> Optional[User]:
    # unwrap because we need the actual object but channels.auth gives us a UserLazyObject
    user = info.context.request.scope["user"]._wrapped
    if isinstance(user, AnonymousUser):
        return None
    return user


@strawberry.type
class Query(ExecutionQuery):
    me: Optional[User] = gql.django.field(resolver=get_me)
    user: Optional[User] = gql.relay.node()
    organization: Optional[Organization] = gql.relay.node()
    owner_by_slug: Optional[Union[User, Organization]] = gql.django.field(
        resolver=get_user_or_organization_by_slug
    )
    project: Optional[Project] = gql.relay.node()
    project_by_slug: Optional[Project] = gql.django.field(
        resolver=models.Project.objects.get_by_slug
    )
    project_version: Optional[ProjectVersion] = gql.relay.node()
    file: Optional[File] = gql.relay.node()


@strawberry.type
class Mutation(
    UserMutation,
    ProjectMutation,
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
