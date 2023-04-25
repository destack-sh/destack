from typing import Optional

from django.contrib.auth.base_user import BaseUserManager
from django.contrib.auth.models import AbstractUser
from django.db import models, transaction
from django.utils.translation import gettext_lazy as _
from django_choices_field import TextChoicesField

from bench.models.organization import (
    Organization,
    OrganizationMembership,
    OrganizationMembershipLevel,
)
from bench.models.owner import OwnerSlug
from bench.models.utils import UUIDModel
from bench.utils.uuidt import MAX_DESCRIPTION_LENGTH


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


class ClientType(models.TextChoices):
    """The type of device/client."""

    DesktopBrowser = "desktop_browser"
    MobileBrowser = "mobile_browser"


class Client(UUIDModel):
    """A user's client (e.g. web browser window) connected to the server."""

    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    last_seen_at = models.DateTimeField(null=True)
    closed_at = models.DateTimeField(null=True)
    user = models.ForeignKey("User", on_delete=models.CASCADE, related_name="clients")
    type = TextChoicesField(choices_enum=ClientType)
    device_name = models.CharField(max_length=256, null=True, blank=True)
    browser_name = models.CharField(max_length=256, null=True, blank=True)
    project_version = models.ForeignKey(
        "ProjectVersion", on_delete=models.CASCADE, null=True, blank=True
    )
