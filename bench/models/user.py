from typing import TYPE_CHECKING, Optional

import requests
import structlog
from django.contrib.auth.base_user import BaseUserManager
from django.contrib.auth.models import AbstractUser
from django.db import models, transaction
from django.utils.translation import gettext_lazy as _

from bench.language.validation import MAX_DESCRIPTION_LENGTH
from bench.models.organization import Organization, OrganizationMembership
from bench.models.owner import OwnerSlug
from bench.models.utils import UUIDModel
from bench.utils.utils import DEBUG, LOCAL

if TYPE_CHECKING:
    from bench.models import Bench, BenchMembership

logger = structlog.get_logger(__name__)


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
        from bench.models.bench import BenchInvite
        from bench.models.organization import OrganizationInvite

        OrganizationInvite.objects.filter(email=email).update(user=user)
        BenchInvite.objects.filter(email=email).update(user=user)

        if not DEBUG and not LOCAL:
            user._create_in_loops()

        return user


class UserStatus(models.TextChoices):
    INITIATED_SIGNUP = "initiated_signup"
    WAITLISTED = "waitlisted"
    ACTIVE = "active"
    SUSPENDED = "suspended"
    DEACTIVATED = "deactivated"


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
    status: models.CharField = models.CharField(max_length=32, choices=UserStatus.choices)
    bot: models.BooleanField = models.BooleanField(default=False)
    description: models.CharField = models.CharField(
        max_length=MAX_DESCRIPTION_LENGTH, blank=True, null=True
    )

    benches: models.QuerySet["Bench"]  # noqa via Project.user
    bench_invites: models.QuerySet["BenchInvite"]  # noqa via BenchInvite.user
    bench_memberships: models.QuerySet["BenchMembership"]  # noqa via BenchMembership.user
    organizations: models.QuerySet["Organization"]  # noqa via Organization.members
    organization_invites: models.QuerySet["BenchInvite"]  # noqa via OrganizationInvite.user
    organization_memberships: models.QuerySet[
        "OrganizationMembership"
    ]  # noqa via OrganizationMembership.user
    notifications: models.QuerySet["Notification"]  # noqa via Notification.user

    objects: UserManager = UserManager()  # type: ignore

    def __str__(self):
        return self.username

    def __repr__(self):
        return f"<User {self.username} {self.status}>"

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
        if not DEBUG and not LOCAL:
            self._update_in_loops()

    def _to_loops_contact(self) -> dict:
        return {
            "id": str(self.id),
            "username": self.owner_slug_id,
            "email": self.email,
            "firstName": self.first_name,
            "lastName": self.last_name,
            "status": self.status,
        }

    def _create_in_loops(self):
        """
        Creates the user in loops.so for email marketing/tx emails.
        Intended for production use only.
        """
        loops_request("POST", "contacts/create", body=self._to_loops_contact())

    def _update_in_loops(self):
        """
        Updates the user in loops.so for email marketing/tx emails.
        Intended for production use only.
        """
        loops_request("POST", "contacts/update", body=self._to_loops_contact())

    class Meta:
        default_manager_name = "objects"


def loops_request(
    method: str,
    path: str,
    body: Optional[dict] = None,
    query_params: Optional[dict] = None,
    headers: Optional[dict] = None,
) -> requests.Response:
    from bench import settings

    if settings.LOOPS_API_KEY is None:
        raise RuntimeError("LOOPS_API_KEY is not set")
    url = f"https://app.loops.so/api/v1/{path}"
    headers = headers or {}
    headers["Authorization"] = f"Bearer {settings.LOOPS_API_KEY}"
    rep = requests.request(method, url, json=body, params=query_params, headers=headers)
    if rep.status_code != 200:
        raise RuntimeError(f"failed to {method} {url}: {rep.text}")
    return rep
