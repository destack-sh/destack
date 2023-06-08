from datetime import datetime, timedelta
from typing import TYPE_CHECKING, Optional

import pytz
from django.contrib.auth.base_user import BaseUserManager
from django.contrib.auth.models import AbstractUser
from django.db import models, transaction
from django.db.models import F, Q
from django.utils.translation import gettext_lazy as _

from bench.models.organization import (
    Organization,
    OrganizationMembership,
    OrganizationMembershipLevel,
)
from bench.models.owner import OwnerSlug
from bench.models.utils import UUIDModel
from bench.msg.messages import ClientData
from bench.utils.uuidt import MAX_DESCRIPTION_LENGTH

if TYPE_CHECKING:
    from bench.models import Project, ProjectVersion


class UserManager(BaseUserManager["User"]):
    use_in_migrations = True

    def get_by_slug(self, username: str):
        return self.get(owner_slug_id=username)

    @transaction.atomic
    def create_user(self, username: str, email: str, full_name: str, **kwargs) -> "User":
        owner_slug = OwnerSlug.objects.create_slug(username)
        user = self.create(
            username=username, email=email, first_name=full_name, owner_slug=owner_slug, **kwargs
        )

        # recover invites that were sent to this email address
        from bench.models.organization import OrganizationInvite

        OrganizationInvite.objects.filter(email=email).update(user=user)

        return user


class User(AbstractUser, UUIDModel):
    """
    A user is an authenticated human working on a program in bench.
    """

    USERNAME_FIELD = "email"
    REQUIRED_FIELDS: list[str] = []

    email: models.EmailField = models.EmailField(_("email address"), unique=True)
    created_at: models.DateTimeField = models.DateTimeField(auto_now_add=True)
    updated_at: models.DateTimeField = models.DateTimeField(auto_now=True)
    owner_slug: models.ForeignKey = models.OneToOneField(
        "OwnerSlug", unique=True, on_delete=models.CASCADE, null=True, related_name="user"
    )
    owner_slug_id: Optional[str]  # noqa via Statement.reference
    completed_signup: models.BooleanField = models.BooleanField(default=False)
    bot: models.BooleanField = models.BooleanField(default=False)
    description: models.CharField = models.CharField(
        max_length=MAX_DESCRIPTION_LENGTH, blank=True, null=True
    )

    projects: models.QuerySet["Project"]  # noqa via Project.user
    invites: models.QuerySet["ProjectInvite"]  # noqa via OrganizationInvite.user
    organizations: models.QuerySet["Organization"]  # noqa via Organization.members
    memberships: models.QuerySet["OrganizationMembership"]  # noqa via OrganizationMembership.user
    notifications: models.QuerySet["Notification"]  # noqa via Notification.user

    objects: UserManager = UserManager()  # type: ignore

    def __str__(self):
        return self.username

    def __repr__(self):
        return f"<User {self.username} {self.id}>"

    @property
    def slug(self) -> str:
        """Should always equal username"""
        return self.owner_slug_id

    def change_username(self, username: str) -> None:
        """Updates the username and the corresponding slug. Must be atomic."""
        # check if atomic
        if not transaction.get_connection().in_atomic_block:
            raise RuntimeError("update_username must be atomic")
        self.username = username
        self.owner_slug.delete()
        # TODO @UX: keep previous slug for redirect for user/org (under prev_slug)
        self.owner_slug = OwnerSlug.objects.create_slug(username)
        self.save()

    def join_organization(
        self,
        organization: Organization,
        level: OrganizationMembershipLevel = OrganizationMembershipLevel.Member,
    ) -> OrganizationMembership:
        membership = OrganizationMembership.objects.create(
            user=self, organization=organization, level=level
        )
        return membership

    class Meta:
        default_manager_name = "objects"


# :ClientTimeouts
CLIENT_ACTIVE_TIMEOUT_SECONDS = 60 * 1  # 1 minute
CLIENT_PRESENT_TIMEOUT_SECONDS = 60 * 60  # 1 hour


class ClientType(models.TextChoices):
    """The type of device/client."""

    DesktopBrowser = "desktop_browser"
    MobileBrowser = "mobile_browser"


class ClientManager(models.Manager):
    def active(
        self,
        organization: Optional["Organization"] = None,
        user: Optional["User"] = None,
        project: Optional["Project"] = None,
        project_version: Optional["ProjectVersion"] = None,
    ) -> models.QuerySet["Client"]:
        active_cutoff = datetime.utcnow().replace(tzinfo=pytz.UTC) - timedelta(
            seconds=CLIENT_ACTIVE_TIMEOUT_SECONDS
        )
        qs = self.filter(
            Q(last_seen_at__gte=active_cutoff)
            & (Q(closed_at__isnull=True) | Q(closed_at__lt=F("last_seen_at")))
        )
        if organization is not None:
            qs = qs.filter(user__memberships__organization=organization)
        if user is not None:
            qs = qs.filter(user=user)
        if project is not None:
            qs = qs.filter(project=project)
        if project_version is not None:
            qs = qs.filter(project_version=project_version)
        return qs


class Client(UUIDModel):
    """A user's client (e.g. web browser window) connected to the server."""

    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    last_seen_at = models.DateTimeField(null=True)
    closed_at = models.DateTimeField(null=True)
    user = models.ForeignKey("User", on_delete=models.CASCADE, related_name="clients")
    type = models.CharField(max_length=32, choices=ClientType.choices)
    device_name = models.CharField(max_length=256, null=True, blank=True)
    browser_name = models.CharField(max_length=256, null=True, blank=True)
    # current location in the app
    project = models.ForeignKey("Project", on_delete=models.SET_NULL, null=True, blank=True)
    project_version = models.ForeignKey(
        "ProjectVersion", on_delete=models.SET_NULL, null=True, blank=True
    )
    file_id = models.UUIDField(null=True, blank=True)
    statement_id = models.UUIDField(null=True, blank=True)
    field_id = models.UUIDField(null=True, blank=True)
    record_id = models.UUIDField(null=True, blank=True)
    path = models.CharField(max_length=256, null=True, blank=True)

    @property
    def active(self) -> bool:
        if self.closed_at is not None and self.closed_at >= self.last_seen_at:
            return False
        active_cutoff = datetime.utcnow().replace(tzinfo=pytz.UTC) - timedelta(
            seconds=CLIENT_ACTIVE_TIMEOUT_SECONDS
        )
        return self.last_seen_at is not None and self.last_seen_at >= active_cutoff

    @property
    def present(self) -> bool:
        if self.closed_at is not None and self.closed_at >= self.last_seen_at:
            return False
        present_cutoff = datetime.utcnow().replace(tzinfo=pytz.UTC) - timedelta(
            seconds=CLIENT_PRESENT_TIMEOUT_SECONDS
        )
        return self.last_seen_at is not None and self.last_seen_at >= present_cutoff

    def __str__(self):
        active_str = "active" if self.active else "inactive"
        return f"{self.user} {self.id} {active_str} ({self.type}, {self.device_name}, {self.browser_name})"

    def __repr__(self):
        return f"<Client {self}>"

    objects = ClientManager()


def rmap_client(client: Client) -> ClientData:
    return ClientData(
        id=client.id,
        created_at=client.created_at,
        last_seen_at=client.last_seen_at,
        closed_at=client.closed_at,
        user_id=client.user_id,
        type=client.type,
        device_name=client.device_name,
        browser_name=client.browser_name,
        project_id=client.project_id,
        project_version_id=client.project_version_id,
        file_id=client.file_id,
        statement_id=client.statement_id,
        field_id=client.field_id,
        record_id=client.record_id,
        path=client.path,
    )


def wmap_client(client_data: ClientData) -> Client:
    return Client(
        id=client_data.id,
        created_at=client_data.created_at,
        last_seen_at=client_data.last_seen_at,
        closed_at=client_data.closed_at,
        user_id=client_data.user_id,
        type=client_data.type,
        device_name=client_data.device_name,
        browser_name=client_data.browser_name,
        project_id=client_data.project_id,
        project_version_id=client_data.project_version_id,
        file_id=client_data.file_id,
        statement_id=client_data.statement_id,
        field_id=client_data.field_id,
        record_id=client_data.record_id,
        path=client_data.path,
    )
