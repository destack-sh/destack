from typing import TYPE_CHECKING, Union

from django.core.validators import validate_slug
from django.db import models

if TYPE_CHECKING:
    from bench.models import Organization, User


class OwnerSlugManager(models.Manager["OwnerSlug"]):
    def get_by_slug(self, slug: str):
        return self.get(slug=slug)

    def create_slug(self, slug: str):
        return self.create(slug=slug)


class OwnerSlug(models.Model):
    """The unique slug of an owner (user or organization)."""

    slug = models.SlugField(primary_key=True, max_length=128, validators=[validate_slug])
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)

    objects = OwnerSlugManager()

    @property
    def owner(self) -> Union["User", "Organization"]:
        try:
            return self.user
        except AttributeError:
            return self.organization
