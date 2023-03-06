import os
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
from bench.api.auth import CanViewProject
from bench.api.deployment import DeploymentMutation
from bench.api.execution import ExecutionQuery
from bench.api.notification import NotificationMutation
from bench.api.organization import Organization, OrganizationMutation
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
from bench.api.token import AccessTokenMutation
from bench.api.user import User, UserFilter, UserMutation
from bench.models import OwnerSlug
from bench.settings import DEBUG, TEST

PyType = typing.Type


@strawberry.type
class SystemInfo:
    version: str
    git_commit: str


SYSTEM_INFO = SystemInfo(
    version=os.environ["VERSION"], git_commit=os.environ.get("GIT_COMMIT", "dev")
)


@sync_to_async
def get_user_or_organization_by_slug(
    self, info: Info, slug: str
) -> Optional[Union[User, Organization]]:
    try:
        slug = OwnerSlug.objects.get(slug=slug)
        return slug.owner
    except OwnerSlug.DoesNotExist:
        return None


def get_me(self, info: Info) -> Optional[User]:
    # unwrap because we need the actual object but channels.auth gives us a UserLazyObject
    user = info.context.request.scope["user"]._wrapped
    if isinstance(user, AnonymousUser):
        return None
    return user


@strawberry.type
class Query(ExecutionQuery):
    system_info: SystemInfo = gql.field(resolver=lambda: SYSTEM_INFO)
    me: Optional[User] = gql.django.field(resolver=get_me)
    user: Optional[User] = gql.relay.node()
    users: gql.relay.Connection[User] = gql.django.connection(filters=UserFilter)
    user_by_slug: Optional[User] = gql.django.field(resolver=models.User.objects.get_by_slug)
    organization: Optional[Organization] = gql.relay.node()
    organizations: gql.relay.Connection[Organization] = gql.django.connection()
    organization_by_slug: Optional[Organization] = gql.django.field(
        resolver=models.Organization.objects.get_by_slug
    )
    owner_by_slug: Optional[Union[User, Organization]] = gql.django.field(
        resolver=get_user_or_organization_by_slug
    )
    project: Optional[Project] = gql.relay.node(directives=[CanViewProject()])
    project_by_slug: Optional[Project] = gql.django.field(
        resolver=models.Project.objects.get_by_slug, directives=[CanViewProject()]
    )
    project_version: Optional[ProjectVersion] = gql.relay.node(directives=[CanViewProject()])
    project_version_by_slug: Optional[ProjectVersion] = gql.django.field(
        resolver=models.ProjectVersion.objects.get_by_slug, directives=[CanViewProject()]
    )
    file: Optional[File] = gql.relay.node(directives=[CanViewProject()])


@strawberry.type
class Mutation(
    UserMutation,
    OrganizationMutation,
    AccessTokenMutation,
    NotificationMutation,
    ProjectMutation,
    ProjectVersionMutation,
    StatementMutation,
    SymbolMutation,
    FileMutation,
    DeploymentMutation,
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
