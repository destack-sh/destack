from typing import Optional, cast

from django.contrib.auth.base_user import BaseUserManager
from django.contrib.auth.models import AbstractUser
from django.db import models
from django.utils.translation import gettext_lazy as _

from bench.models.utils import UUIDModel


class UserManager(BaseUserManager[AbstractUser]):
    use_in_migrations = True

    def create_user(
        self, email: str, password: Optional[str], first_name: str, **extra_fields
    ) -> "User":
        email = self.normalize_email(email)
        user = self.model(email=email, first_name=first_name, **extra_fields)
        if password is not None:
            user.set_password(password)
        user.save()
        return cast(User, user)


class User(AbstractUser, UUIDModel):
    USERNAME_FIELD = "email"
    REQUIRED_FIELDS: list[str] = []

    created_at: models.DateTimeField = models.DateTimeField(auto_now_add=True)
    updated_at: models.DateTimeField = models.DateTimeField(auto_now=True)
    email: models.EmailField = models.EmailField(_("email address"), unique=True)

    objects: UserManager = UserManager()  # type: ignore
