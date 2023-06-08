from __future__ import annotations

import secrets
from datetime import datetime
from hashlib import sha256
from typing import TYPE_CHECKING, Union

from django.contrib.postgres.fields import ArrayField
from django.db import models

from bench.models.utils import UUIDModel
from bench.settings import ACCESS_TOKEN_DIGEST_LENGTH, ACCESS_TOKEN_KEY_LENGTH, ACCESS_TOKEN_PREFIX
from bench.utils.uuidt import MAX_NAME_LENGTH

if TYPE_CHECKING:
    from bench.models import Organization, User


def digest_raw_token(raw_token):
    # sha256 is fine here because we're using it for HMAC
    # noinspection InsecureHash
    return sha256(raw_token.encode()).hexdigest()


class AccessTokenManager(models.Manager["AccessToken"]):
    def create_token(
        self,
        owner: Union["User", "Organization"],
        name: str | None,
        scopes: list[AccessTokenScope],
        expires_at: datetime | None,
    ) -> tuple[AccessToken, str]:
        """Creates a new access token for the owner, returning the ephemeral raw token (not saved)."""
        # generate secure random string for token key (base64 encoded)
        raw_token = ACCESS_TOKEN_PREFIX + secrets.token_hex(ACCESS_TOKEN_DIGEST_LENGTH // 8)
        token_key = raw_token[-ACCESS_TOKEN_KEY_LENGTH:]
        digest = digest_raw_token(raw_token)

        from bench.models import Organization, User  # prevent circular import

        access_token = self.create(
            digest=digest,
            token_key=token_key,
            name=name,
            scopes=scopes,
            expires_at=expires_at,
            organization=owner if isinstance(owner, Organization) else None,
            user=owner if isinstance(owner, User) else None,
        )
        return access_token, raw_token


class AccessTokenScope(models.TextChoices):
    RUN = "run"


class AccessTokenStatus(models.TextChoices):
    ACTIVE = "active"
    EXPIRED = "expired"
    REVOKED = "revoked"


class AccessToken(UUIDModel):
    digest = models.CharField(max_length=ACCESS_TOKEN_DIGEST_LENGTH, unique=True)
    token_key = models.CharField(max_length=ACCESS_TOKEN_KEY_LENGTH)
    scopes = ArrayField(models.CharField(max_length=32, choices=AccessTokenScope.choices))
    name = models.CharField(max_length=MAX_NAME_LENGTH, null=True)
    secret = models.ForeignKey("Secret", on_delete=models.CASCADE, null=True)

    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    expires_at = models.DateTimeField(null=True)
    revoked_at = models.DateTimeField(null=True)

    organization: models.ForeignKey = models.ForeignKey(
        "Organization", on_delete=models.CASCADE, null=True, related_name="access_tokens"
    )
    user = models.ForeignKey(
        "User", on_delete=models.CASCADE, null=True, related_name="access_tokens"
    )

    def __str__(self):
        return f"{self.owner}/{ACCESS_TOKEN_PREFIX}...{self.token_key} ({self.status}, {self.name or '<unnamed>'}, {self.scopes})"

    def __repr__(self):
        return f"<AccessToken {self}>"

    def revoke(self):
        self.revoked_at = datetime.utcnow()
        self.save()

    @property
    def status(self) -> AccessTokenStatus:
        if self.revoked:
            return AccessTokenStatus.REVOKED
        elif self.expired:
            return AccessTokenStatus.EXPIRED
        else:
            return AccessTokenStatus.ACTIVE

    @property
    def active(self) -> bool:
        return not self.expired and not self.revoked

    @property
    def expired(self):
        return self.expires_at is not None and datetime.utcnow() >= self.expires_at

    @property
    def revoked(self):
        return self.revoked_at is not None

    @property
    def owner(self) -> Organization | User:
        return self.organization or self.user

    objects = AccessTokenManager()

    class Meta:
        ordering = ["-created_at"]
        constraints = [
            # at least one of organization or user must be set
            models.CheckConstraint(
                check=models.Q(organization__isnull=False) | models.Q(user__isnull=False),
                name="bench_access_token_has_owner",
            ),
            # owner plus token key must be unique
            models.UniqueConstraint(
                fields=["organization", "token_key"],
                condition=models.Q(organization__isnull=False),
                name="bench_access_token_organization_token_key_ak",
            ),
            models.UniqueConstraint(
                fields=["user", "token_key"],
                condition=models.Q(user__isnull=False),
                name="bench_access_token_user_token_key_ak",
            ),
        ]
