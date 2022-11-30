from typing import Optional

from django.contrib.auth.base_user import BaseUserManager
from django.contrib.auth.models import AbstractUser
from django.db import models
from django.utils.translation import gettext_lazy as _

from bench.models import Organization
from bench.models.organization import OrganizationMembership
from bench.models.utils import UUIDModel


class UserManager(BaseUserManager["User"]):
    use_in_migrations = True

    def create_user(self, email: str, password: Optional[str], first_name: str, **kwargs) -> "User":
        email = self.normalize_email(email)
        user = self.model(email=email, first_name=first_name, **kwargs)
        if password is not None:
            user.set_password(password)
        user.save()
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

    objects: UserManager = UserManager()  # type: ignore

    def join_organization(
        self, organization: Organization, level: OrganizationMembership.Level
    ) -> OrganizationMembership:
        membership = OrganizationMembership.objects.create(
            user=self, organization=organization, level=level
        )
        return membership

    class Meta:
        default_manager_name = "objects"
