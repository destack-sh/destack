from typing import TYPE_CHECKING, Annotated, Optional, cast

from asgiref.sync import async_to_sync
from channels.auth import logout as channels_logout
from django.core.exceptions import PermissionDenied
from strawberry import lazy
from strawberry.types import Info
from strawberry_django_plus import gql
from strawberry_django_plus.gql import auto
from strawberry_django_plus.types import OperationInfo

from bench import models
from bench.api.auth import CanViewProject, CanWriteUser, can_write_user, check_can_write_user
from bench.api.notification import Notification, NotificationFilter
from bench.api.owner import AccessTokenFilter, Owner
from bench.api.util import safe_mutation

if TYPE_CHECKING:
    from bench.api.organization import Organization, OrganizationMembership
    from bench.api.project import Project
    from bench.api.token import AccessToken


@gql.django.filter(models.User)
class UserFilter:
    slug_prefix: Optional[str]
    email_equals: Optional[str]

    def filter(self, queryset):
        if self.slug_prefix:
            queryset = queryset.filter(username__startswith=self.slug_prefix)
        if self.email_equals:
            queryset = queryset.filter(email=self.email_equals)
        return queryset


@gql.django.type(models.User)
class User(gql.relay.Node, Owner):
    username: auto
    email: auto
    created_at: auto
    updated_at: auto
    completed_signup: auto
    bot: auto
    description: auto

    organizations: gql.relay.Connection[
        Annotated["Organization", lazy(".organization")]
    ] = gql.django.connection()
    organization_memberships: gql.relay.Connection[
        Annotated["OrganizationMembership", lazy(".organization")]
    ] = gql.django.connection()
    projects: gql.relay.Connection[Annotated["Project", lazy(".project")]] = gql.django.connection(
        directives=[CanViewProject(at_root=False)]
    )
    access_tokens: gql.relay.Connection[
        Annotated["AccessToken", lazy(".token")]
    ] = gql.django.connection(filters=AccessTokenFilter, directives=[CanWriteUser(at_root=False)])
    notifications: gql.relay.Connection["Notification"] = gql.django.connection(
        filters=NotificationFilter, directives=[CanWriteUser(at_root=False)]
    )

    @gql.field
    def can_view_full(self, info: OperationInfo):
        user = cast(models.User, info.context.request.scope["user"]._wrapped)
        return can_write_user(user, self)

    @gql.field
    def can_write(self, info: OperationInfo):
        user = cast(models.User, info.context.request.scope["user"]._wrapped)
        return can_write_user(user, self)

    @gql.django.field(only=["first_name"])
    def name(self) -> str:
        return self.first_name

    @gql.django.field(only=["owner_slug_id"])
    def slug(self, info) -> str:
        return self.owner_slug_id


@gql.input
class UserCompleteSignupInput(gql.NodeInput):
    username: str
    full_name: str


@gql.input
class UserUpdateInput(gql.NodeInput):
    name: str
    description: str


@gql.input
class UserRenameInput(gql.NodeInput):
    slug: str


@gql.type
class UserMutation:
    # there's no create user mutation here because we only support social auth for now

    @safe_mutation(atomic=True)
    def complete_signup(self, info, input: UserCompleteSignupInput) -> User | OperationInfo:
        user = models.User.objects.get(id=input.id.node_id)
        requesting_user = info.context.request.scope["user"]
        if not requesting_user.is_authenticated or user.id != requesting_user.id:
            raise PermissionDenied("can only complete signup for yourself")
        user.change_username(input.username)
        user.first_name = input.full_name
        user.completed_signup = True
        user.save()
        return user

    @safe_mutation
    def update_user(self, info, input: UserUpdateInput) -> User | OperationInfo:
        user = models.User.objects.get(id=input.id.node_id)
        check_can_write_user(info, user)
        user.first_name = input.name
        user.description = input.description
        user.save()
        return user

    @safe_mutation
    def logout(self, info: Info) -> None | OperationInfo:
        if not info.context.request.scope["user"].is_authenticated:
            raise PermissionDenied("can only logout when logged in")
        async_to_sync(channels_logout)(info.context.request.scope)
        return None
