import os
import typing
from typing import TYPE_CHECKING, Annotated, List, Optional, Union

import strawberry
import strawberry_django
from django.contrib.auth.models import AnonymousUser
from graphql import GraphQLError, NoSchemaIntrospectionCustomRule
from strawberry import lazy
from strawberry.extensions import AddValidationRules, Extension, ParserCache, QueryDepthLimiter
from strawberry.relay import GlobalID
from strawberry.types import ExecutionContext, Info
from strawberry_django.optimizer import DjangoOptimizerExtension

from bench import models
from bench.api.auth import HasModuleAccess
from bench.api.dataset import DatasetMutation, RecordQuery
from bench.api.file import File, FileMutation
from bench.api.module import read_module_node_by_id
from bench.api.multiplayer import MultiplayerSubscription
from bench.api.notification import NotificationMutation
from bench.api.object import ObjectMutation, RemoteObject
from bench.api.organization import Organization, OrganizationMutation
from bench.api.project import (
    Project,
    ProjectMutation,
    ProjectVersion,
    ProjectVersionMutation,
    ProjectVisibility,
)
from bench.api.secret import Secret, SecretMutation
from bench.api.sentry import SentryPerformanceExtension
from bench.api.session import SessionMutation, SessionQuery, SessionSubscription
from bench.api.statement import StatementMutation, SymbolMutation
from bench.api.token import AccessTokenMutation
from bench.api.user import ClientQuery, ClientSubscription, User, UserFilter, UserMutation
from bench.api.utils import HasCrud, get_user_from_info
from bench.models import ModuleAccessLevel, OwnerSlug
from bench.settings import DEBUG, TEST
from bench.utils.utils import sentry_capture

if TYPE_CHECKING:
    from bench.api.statement import Statement

PyType = typing.Type


@strawberry.type
class SystemInfo:
    version: str
    git_commit: str


SYSTEM_INFO = SystemInfo(
    version=os.environ["VERSION"], git_commit=os.environ.get("GIT_COMMIT", "dev")
)


def get_me(self, info: Info) -> Optional[User]:
    # unwrap because we need the actual object but channels.auth gives us a UserLazyObject
    user = get_user_from_info(info)
    if isinstance(user, AnonymousUser):
        return None
    return user


def get_user_or_organization_by_slug(
    self, info: Info, slug: str
) -> Optional[Union[User, Organization]]:
    try:
        slug = OwnerSlug.objects.get(slug=slug)
        return slug.owner
    except OwnerSlug.DoesNotExist:
        return None


def get_project_version_by_tag(project_id: GlobalID, tag: str):
    try:
        return models.ProjectVersion.objects.get_by_tag(project_id.node_id, tag)
    except models.ProjectVersion.DoesNotExist:
        return None


def get_project_version_by_slug(owner: str, project: str, tag: str):
    try:
        return models.ProjectVersion.objects.get_by_slug(owner, project, tag)
    except models.ProjectVersion.DoesNotExist:
        return None


def get_project_by_slug(owner: str, project: str):
    try:
        return models.Project.objects.get_by_slug(owner, project)
    except models.Project.DoesNotExist:
        return None


def get_user_by_slug(slug: str):
    try:
        return models.User.objects.get_by_slug(slug)
    except models.User.DoesNotExist:
        return None


def get_organization_by_slug(organization: str):
    try:
        return models.Organization.objects.get_by_slug(organization)
    except models.Organization.DoesNotExist:
        return None


def get_featured_projects(self) -> typing.Iterable[Project]:
    # just return symbolx projects for now
    return models.Project.objects.filter(
        organization__owner_slug_id="symbolx", visibility=ProjectVisibility.PUBLIC
    )


@strawberry.type
class Query(SessionQuery, ClientQuery, RecordQuery):
    system_info: SystemInfo = strawberry_django.field(resolver=lambda: SYSTEM_INFO)

    # users/orgs
    me: Optional[User] = strawberry_django.field(resolver=get_me)
    user: Optional[User] = strawberry_django.node()
    users: strawberry_django.relay.ListConnectionWithTotalCount[
        User
    ] = strawberry_django.connection(filters=UserFilter)
    organization: Optional[Organization] = strawberry_django.node()
    owner_by_slug: Optional[Union[User, Organization]] = strawberry_django.field(
        resolver=get_user_or_organization_by_slug
    )

    # project
    project: Optional[Project] = strawberry_django.node(extensions=[HasModuleAccess()])
    project_by_slug: Optional[Project] = strawberry_django.field(
        resolver=get_project_by_slug, extensions=[HasModuleAccess()]
    )
    project_version: Optional[ProjectVersion] = strawberry_django.node()
    project_version_by_slug: Optional[ProjectVersion] = strawberry_django.field(
        resolver=get_project_version_by_slug, extensions=[HasModuleAccess()]
    )
    project_version_by_tag: Optional[ProjectVersion] = strawberry_django.field(
        resolver=get_project_version_by_tag, extensions=[HasModuleAccess()]
    )
    featured_projects: strawberry_django.relay.ListConnectionWithTotalCount[
        Project
    ] = strawberry_django.connection(resolver=get_featured_projects)

    # module
    module: Optional[ProjectVersion] = strawberry_django.field(resolver=read_module_node_by_id)
    file: Optional[File] = strawberry_django.field(resolver=read_module_node_by_id)
    statement: Optional[Annotated["Statement", lazy(".statement")]] = strawberry_django.field(
        resolver=read_module_node_by_id
    )
    remote_object: Optional[RemoteObject] = strawberry_django.node(
        extensions=[HasModuleAccess(map=lambda obj: obj.project)]
    )
    secret: Optional[Secret] = strawberry_django.node(
        extensions=[HasModuleAccess(map=lambda obj: obj.project, level=ModuleAccessLevel.Edit)]
    )


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
    DatasetMutation,
    FileMutation,
    SessionMutation,
    ObjectMutation,
    SecretMutation,
):
    pass


@strawberry.type
class Subscription(
    ClientSubscription,
    MultiplayerSubscription,
    SessionSubscription,
):
    pass


default_extensions: list[Union[PyType[Extension], Extension]] = [
    DjangoOptimizerExtension,
    QueryDepthLimiter(max_depth=10),
    SentryPerformanceExtension,
]
prod_extensions: list[Union[PyType[Extension], Extension]] = [
    ParserCache(),
    AddValidationRules([NoSchemaIntrospectionCustomRule]),
]
if DEBUG or TEST:
    extensions = default_extensions
else:
    extensions = default_extensions + prod_extensions


class SentryCaptureSchema(strawberry.Schema):
    def process_errors(
        self, errors: List[GraphQLError], execution_context: Optional[ExecutionContext] = None
    ) -> None:
        for error in errors:
            if error.original_error:
                sentry_capture(error.original_error)
        super().process_errors(errors, execution_context)


schema = SentryCaptureSchema(
    Query,
    Mutation,
    Subscription,
    extensions=extensions,
    types=[HasCrud],
)
