from typing import TYPE_CHECKING, Annotated, Optional
from uuid import UUID

import strawberry
import strawberry_django
import structlog
from asgiref.sync import async_to_sync
from channels.auth import login as channels_login
from channels.auth import logout as channels_logout
from django.core.exceptions import PermissionDenied
from strawberry import auto, lazy, relay
from strawberry.relay import GlobalID
from strawberry.types import Info
from strawberry_django.fields.types import OperationInfo

from bench import models
from bench.api.auth import CheckTarget, IsUser, can_write_user, check_can_write_user
from bench.api.notification import Notification, NotificationFilter
from bench.api.owner import AccessTokenFilter, Owner
from bench.api.utils import get_user_from_info, safe_mutation

if TYPE_CHECKING:
    from bench.api.bench import Bench, BenchVersion
    from bench.api.organization import Organization, OrganizationMembership
    from bench.api.token import AccessToken

logger = structlog.get_logger(__name__)


@strawberry_django.filter(models.User)
class UserFilter:
    slug_prefix: Optional[str]
    email_equals: Optional[str]

    def filter(self, queryset):
        if self.slug_prefix:
            queryset = queryset.filter(username__startswith=self.slug_prefix)
        if self.email_equals:
            queryset = queryset.filter(email=self.email_equals)
        return queryset


UserStatus = strawberry.enum(models.UserStatus)


@strawberry_django.type(models.User)
class User(Owner, relay.Node):
    username: auto
    email: auto
    created_at: auto
    updated_at: auto
    status: UserStatus
    bot: auto
    description: auto

    organizations: strawberry_django.relay.ListConnectionWithTotalCount[
        Annotated["Organization", lazy(".organization")]
    ] = strawberry_django.connection()
    organization_memberships: strawberry_django.relay.ListConnectionWithTotalCount[
        Annotated["OrganizationMembership", lazy(".organization")]
    ] = strawberry_django.connection()
    benches: strawberry_django.relay.ListConnectionWithTotalCount[
        Annotated["Bench", lazy(".bench")]
    ] = strawberry_django.connection(extensions=[IsUser(target=CheckTarget.ROOT)])
    access_tokens: strawberry_django.relay.ListConnectionWithTotalCount[
        Annotated["AccessToken", lazy(".token")]
    ] = strawberry_django.connection(
        filters=AccessTokenFilter, extensions=[IsUser(target=CheckTarget.ROOT)]
    )
    notifications: strawberry_django.relay.ListConnectionWithTotalCount[
        "Notification"
    ] = strawberry_django.connection(
        filters=NotificationFilter, extensions=[IsUser(target=CheckTarget.ROOT)]
    )

    @strawberry_django.field
    def can_view_detail(self, info: Info) -> bool:
        return can_write_user(info, self)

    @strawberry_django.field
    def can_write(self, info: Info) -> bool:
        return can_write_user(info, self)

    @strawberry_django.field(only=["first_name"])
    def name(self) -> str:
        return self.first_name

    @strawberry_django.field(only=["owner_slug_id"])
    def slug(self, info) -> str:
        return self.owner_slug_id


ClientType = strawberry.enum(models.ClientType)


@strawberry_django.type(models.Client)
class Client(relay.Node):
    created_at: auto
    updated_at: auto
    last_seen_at: auto
    closed_at: auto
    type: ClientType
    device_name: auto
    browser_name: auto
    user: Annotated["User", lazy(".user")]
    bench: Optional[Annotated["Bench", lazy(".bench")]]
    bench_version: Optional[Annotated["BenchVersion", lazy(".bench")]]
    file_id: Optional[UUID]
    statement_id: Optional[UUID]
    path: auto
    active: bool
    present: bool


@strawberry.input
class UserCompleteSignupInput(strawberry_django.NodeInput):
    username: str
    full_name: str


@strawberry.input
class UserUpdateInput(strawberry_django.NodeInput):
    name: str
    description: str


@strawberry.input
class UserRenameInput(strawberry_django.NodeInput):
    slug: str


@strawberry.type
class UserMutation:
    # there's no create user mutation here because we only support social auth for now

    @safe_mutation(atomic=True)
    def complete_signup(self, info, input: UserCompleteSignupInput) -> User | OperationInfo:
        user = models.User.objects.get(id=input.id.node_id)
        requesting_user = get_user_from_info(info)
        if not requesting_user.is_authenticated or user.id != requesting_user.id:
            raise PermissionDenied("can only complete signup for yourself")
        user.change_username(input.username)
        user.first_name = input.full_name
        user.status = UserStatus
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
    def accept_organization_invite(self, info, id: GlobalID) -> User | OperationInfo:
        invite = models.OrganizationInvite.objects.get(id=id.node_id)
        check_can_write_user(info, invite.user)
        invite.accept()
        return invite.user

    @safe_mutation
    def accept_bench_invite(self, info, id: GlobalID) -> User | OperationInfo:
        invite = models.BenchInvite.objects.get(id=id.node_id)
        check_can_write_user(info, invite.user)
        invite.accept()
        return invite.user

    @safe_mutation
    def logout(self, info: Info) -> None | OperationInfo:
        if not get_user_from_info(info).is_authenticated:
            raise PermissionDenied("can only logout when logged in")
        async_to_sync(channels_logout)(info.context["request"].consumer.scope)
        return None

    # TODO @Security!: check that secret root login is never exposed in prod
    @safe_mutation
    def secret_root_login(self, info: Info, username: str) -> User | OperationInfo:
        from bench.utils.utils import DEBUG, TEST

        if not (TEST or DEBUG):
            raise PermissionDenied("can only use this in test mode")
        user = models.User.objects.get(username=username)
        async_to_sync(channels_login)(
            info.context["request"].consumer.scope,
            user,
            backend="django.contrib.auth.backends.ModelBackend",
        )
        return user
