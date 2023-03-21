import random
import re
from typing import TYPE_CHECKING, Union

from django.core.validators import RegexValidator
from django.db import models

if TYPE_CHECKING:
    from bench.models import Organization, User

SLUG_REGEX = re.compile(r"^[a-z0-9_-]{3,}\Z")
SLUG_VALIDATOR = RegexValidator(regex=SLUG_REGEX, message="Invalid slug")


def slugify(value: str) -> str:
    if SLUG_REGEX.match(value):
        return value
    if not value:
        value = "user"
    # replace non-slug characters with dashes
    value = re.sub(r"[^a-z0-9_-]", "-", value.lower())
    # extend dashes to a reasonable length
    if len(value) <= 4:
        value += str(random.randint(100000, 999999))
    return value


class OwnerSlugManager(models.Manager["OwnerSlug"]):
    def get_by_slug(self, slug: str):
        return self.get(slug=slug)

    def create_slug(self, slug: str):
        return self.create(slug=slug)


class OwnerSlug(models.Model):
    """The unique slug of an owner (user or organization)."""

    slug = models.SlugField(primary_key=True, max_length=128, validators=[SLUG_VALIDATOR])
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)

    objects = OwnerSlugManager()

    def __str__(self):
        return self.slug

    def __repr__(self):
        return f"<OwnerSlug {self.slug}>"

    @property
    def owner(self) -> Union["User", "Organization"]:
        try:
            return self.user
        except AttributeError:
            try:
                return self.organization
            except AttributeError:
                raise RuntimeError(f"{self} has no owner")
