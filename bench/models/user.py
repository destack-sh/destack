from typing import Optional

from django.contrib.auth.base_user import BaseUserManager
from django.contrib.auth.models import AbstractUser
from django.db import models, transaction
from django.utils.translation import gettext_lazy as _

from bench.models.organization import Organization, OrganizationMembership
from bench.models.owner import OwnerSlug
from bench.models.utils import UUIDModel


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

    objects: UserManager = UserManager()  # type: ignore

    def __str__(self):
        return self.username

    def __repr__(self):
        return f"<User {self.username} {self.id}>"

    @property
    def slug(self) -> str:
        """Should always equal username"""
        return self.owner_slug_id

    def join_organization(
        self, organization: Organization, level: OrganizationMembership.Level
    ) -> OrganizationMembership:
        membership = OrganizationMembership.objects.create(
            user=self, organization=organization, level=level
        )
        return membership

    class Meta:
        default_manager_name = "objects"
