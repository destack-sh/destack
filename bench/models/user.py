from typing import Optional, cast

from django.contrib.auth.base_user import BaseUserManager
from django.contrib.auth.models import AbstractUser
from django.db import models, transaction
from django.utils.translation import gettext_lazy as _

from bench.models import Organization
from bench.models.organization import OrganizationMembership
from bench.models.utils import UUIDModel


class UserManager(BaseUserManager[AbstractUser]):
    use_in_migrations = True

    def bootstrap(
        self,
        email: str,
        password: Optional[str],
        first_name: str,
        organization_name: str,
        organization_kwargs: Optional[dict] = None,
        team_kwargs: Optional[dict] = None,
        user_kwargs: Optional[dict] = None,
        is_staff: bool = False,
    ) -> tuple[Organization, "User"]:
        organization_kwargs = organization_kwargs or {}
        team_kwargs = team_kwargs or {}
        user_kwargs = user_kwargs or {}

        with transaction.atomic():
            organization = Organization.objects.create(
                name=organization_name, **organization_kwargs
            )
            user = self.create_user(
                email=email,
                password=password,
                first_name=first_name,
                is_staff=is_staff,
                **user_kwargs
            )
            user.join_organization(organization, level=OrganizationMembership.Level.Owner)

        return organization, user

    def create_user(self, email: str, password: Optional[str], first_name: str, **kwargs) -> "User":
        email = self.normalize_email(email)
        user = self.model(email=email, first_name=first_name, **kwargs)
        if password is not None:
            user.set_password(password)
        user.save()
        return cast(User, user)


class User(AbstractUser, UUIDModel):
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
