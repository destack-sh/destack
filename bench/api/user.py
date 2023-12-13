from datetime import timedelta
from typing import TYPE_CHECKING, Annotated, AsyncGenerator, Iterable, Optional
from uuid import UUID

import strawberry
import strawberry_django
import structlog
from asgiref.sync import async_to_sync
from channels.auth import login as channels_login
from channels.auth import logout as channels_logout
from django.core.exceptions import PermissionDenied
from django.db.models import F, Q
from strawberry import auto, lazy, relay
from strawberry.channels.handlers.http_handler import ChannelsRequest
from strawberry.channels.handlers.ws_handler import GraphQLWSConsumer
from strawberry.relay import GlobalID
from strawberry.types import Info
from strawberry_django.fields.types import OperationInfo

from bench import models
from bench.api.auth import CheckTarget, IsUser, can_write_user, check_can_write_user
from bench.api.notification import Notification, NotificationFilter
from bench.api.owner import AccessTokenFilter, Owner
from bench.api.utils import asafe_subscription, get_user_from_info, safe_mutation, to_uuid
from bench.models.user import (
    CLIENT_ACTIVE_TIMEOUT_SECONDS,
    CLIENT_PRESENT_TIMEOUT_SECONDS,
    pack_client,
    unpack_client,
)
from bench.msg.core import NMessage, publish_soon, subscribe
from bench.msg.messages import ClientChangedPayload, ClientOrigin, NMessageType
from bench.utils.dt import utcnow_with_tz

if TYPE_CHECKING:
    from bench.api.organization import Organization, OrganizationMembership
    from bench.api.project import Project, ProjectVersion
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
    projects: strawberry_django.relay.ListConnectionWithTotalCount[
        Annotated["Project", lazy(".project")]
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
    project: Optional[Annotated["Project", lazy(".project")]]
    project_version: Optional[Annotated["ProjectVersion", lazy(".project")]]
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


@strawberry.input
class ClientUpsertInput(strawberry_django.NodeInput):
    type: ClientType
    device_name: Optional[str]
    browser_name: Optional[str]
    project_id: Optional[GlobalID]
    project_version_id: Optional[GlobalID]
    file_id: Optional[GlobalID]
    statement_id: Optional[GlobalID]
    field_id: Optional[GlobalID]
    record_id: Optional[GlobalID]
    path: Optional[str]


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
    def accept_project_invite(self, info, id: GlobalID) -> User | OperationInfo:
        invite = models.ProjectInvite.objects.get(id=id.node_id)
        check_can_write_user(info, invite.user)
        invite.accept()
        return invite.user

    @safe_mutation
    def logout(self, info: Info) -> None | OperationInfo:
        if not get_user_from_info(info).is_authenticated:
            raise PermissionDenied("can only logout when logged in")
        # also close client
        client_id = _get_client_id(info)
        if client_id is not None:
            client = models.Client.objects.get(id=client_id)
            client.closed_at = utcnow_with_tz()
            client.last_seen_at = client.closed_at
            client.save()
            _publish_client_changed(client, info)
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

    @safe_mutation
    def upsert_client(self, info: Info, input: ClientUpsertInput) -> Client | OperationInfo:
        user = get_user_from_info(info)
        if not user.is_authenticated:
            raise PermissionDenied("can only upsert client when logged in")
        client, _ = models.Client.objects.get_or_create(
            id=input.id.node_id,
            user=user,
            defaults={"type": input.type, "device_name": input.device_name},
        )
        client.type = input.type
        client.device_name = input.device_name
        client.browser_name = input.browser_name
        client.project_id = input.project_id.node_id if input.project_id else None
        client.project_version_id = (
            input.project_version_id.node_id if input.project_version_id else None
        )
        client.file_id = input.file_id.node_id if input.file_id else None
        client.statement_id = input.statement_id.node_id if input.statement_id else None
        client.field_id = input.field_id.node_id if input.field_id else None
        client.record_id = input.record_id.node_id if input.record_id else None
        client.name = input.path
        client.last_seen_at = utcnow_with_tz()
        client.save()
        # update client id in session if needed
        if _get_client_id(info) != client.id:
            _set_client_id(info, client.id)
        # broadcast client change
        _publish_client_changed(client, info)
        return client

    @safe_mutation
    def close_client(self, info: Info) -> None | Client | OperationInfo:
        user = get_user_from_info(info)
        if not user.is_authenticated:
            raise PermissionDenied("can only close client when logged in")
        client_id = _get_client_id(info)
        if client_id is not None:
            return None  # ignore
        client = models.Client.objects.get(id=client_id)
        client.closed_at = utcnow_with_tz()
        client.last_seen_at = client.closed_at
        client.save()
        _publish_client_changed(client, info)
        return client

    @safe_mutation
    def update_presence(self, info: Info) -> Client | OperationInfo:
        user = get_user_from_info(info)
        if not user.is_authenticated:
            raise PermissionDenied("can only update presence when logged in")
        client_id = _get_client_id(info)
        if client_id is None:
            raise PermissionDenied("can only update presence when client_id is set")
        client = models.Client.objects.get(id=client_id)
        client.last_seen_at = utcnow_with_tz()
        client.save()
        _publish_client_changed(client, info)
        return client


def _get_client_id(info: Info) -> Optional[UUID]:
    request = info.context["request"]
    if isinstance(request, GraphQLWSConsumer):
        client_id = request.scope["session"].get("client_id")
    elif isinstance(request, ChannelsRequest):
        client_id = request.consumer.scope["session"].get("client_id")
    else:
        raise TypeError(f"unexpected request type: {type(request)}")
    if client_id is None:
        return None
    return UUID(client_id)


def _set_client_id(info: Info, client_id: UUID) -> None:
    # TODO @Robustness: modifying session data causes the request to stall (and never return)
    #  This seems to be broken in channels.sessions.SessionMiddleware, but I couldn't figure how.
    #  In practice this means the first upsert client request will hang and
    #  clog one client connection per host for a while (minutes?).
    #  This isn't great and should be addressed but it doesn't affect the UX.
    if isinstance(info.context["request"], GraphQLWSConsumer):
        info.context["request"].scope["session"]["client_id"] = str(client_id)
    elif isinstance(info.context["request"], ChannelsRequest):
        info.context["request"].consumer.scope["session"]["client_id"] = str(client_id)
    else:
        raise TypeError(f"unexpected request type: {type(info.context['request'])}")


def _publish_client_changed(client: models.Client, info: Info):
    # TODO @Performance: client/presence info should live in Redis
    #  (and status changes should contain the entire data, so no reads are required after initial)
    client_nonce = info.context["request"].headers.get("x-client-nonce")
    origin = ClientOrigin("user", client.id, client_nonce)
    client_data = pack_client(client)
    publish_soon(
        NMessageType.CLIENT_CHANGED, ClientChangedPayload(origin=origin, client=client_data)
    )


@strawberry.type
class ClientQuery:
    @strawberry_django.connection(strawberry_django.relay.ListConnectionWithTotalCount[Client])
    async def clients(
        self,
        info: Info,
        project_id: GlobalID | None = None,
        project_version_id: GlobalID | None = None,
        user_id: GlobalID | None = None,
        organization_id: GlobalID | None = None,
        in_same_organizations: bool = True,
        active: Optional[bool] = None,
        present: Optional[bool] = True,
    ) -> Iterable[Client]:
        qs = models.Client.objects.all()
        user = get_user_from_info(info)
        if not user.is_authenticated:
            return qs.none()

        project_id = to_uuid(project_id)
        project_version_id = to_uuid(project_version_id)
        user_id = to_uuid(user_id)
        organization_id = to_uuid(organization_id)
        organization_ids = None
        if in_same_organizations:
            organization_ids = [id async for id in user.organizations.values_list("id", flat=True)]
        if organization_id:
            organization_ids = [organization_id]

        if project_id:
            qs = qs.filter(project_id=project_id)
        if project_version_id:
            qs = qs.filter(project_version_id=project_version_id)
        if user_id:
            qs = qs.filter(user_id=user_id)
        if organization_ids:
            qs = qs.filter(user__organizations__id__in=organization_ids)
        if active:
            active_cutoff = utcnow_with_tz() - timedelta(seconds=CLIENT_ACTIVE_TIMEOUT_SECONDS)
            qs = qs.filter(
                Q(last_seen_at__gte=active_cutoff)
                & (Q(closed_at__isnull=True) | Q(closed_at__lt=F("last_seen_at")))
            )
        if present:
            present_cutoff = utcnow_with_tz() - timedelta(seconds=CLIENT_PRESENT_TIMEOUT_SECONDS)
            qs = qs.filter(
                Q(last_seen_at__gte=present_cutoff)
                & (Q(closed_at__isnull=True) | Q(closed_at__lt=F("last_seen_at")))
            )

        return qs


@strawberry.type
class ClientSubscription:
    @asafe_subscription
    async def clients_changed(
        self,
        info: Info,
        project_id: GlobalID | None,
        project_version_id: GlobalID | None,
    ) -> AsyncGenerator[Client, None]:
        user = get_user_from_info(info)
        project_id = to_uuid(project_id)
        project_version_id = to_uuid(project_version_id)
        client_id = _get_client_id(info)
        client_nonce = to_uuid(info.context["connection_params"].get("X-Client-Nonce"))
        log = logger.bind(user=user, project_version_id=project_version_id, client_id=client_id)

        change_sub = await subscribe(NMessageType.CLIENT_CHANGED, payload_t=ClientChangedPayload)

        log.info("clients.listen")
        while True:
            change: NMessage[ClientChangedPayload] = await change_sub.next_msg()
            if (
                change.payload.origin.id == client_id
                and change.payload.origin.nonce == client_nonce
            ):
                continue  # skip self

            client = unpack_client(change.payload.client)
            if project_id and client.project_id != project_id:
                continue
            if project_version_id and client.project_version_id != project_version_id:
                continue
            log.debug("clients.update", client=client.id)
            yield client
