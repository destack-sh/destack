from django.core.validators import validate_slug
from django.db import models
from social_core.utils import slugify

from bench.models.utils import UUIDModel


class OrganizationManager(models.Manager):
    def create(self, **kwargs) -> "Organization":
        if "slug" not in kwargs:
            slug = slugify(kwargs["name"])
            kwargs["slug"] = slug
        return super().create(**kwargs)


class Organization(UUIDModel):
    name: models.CharField = models.CharField(max_length=256)
    slug: models.SlugField = models.SlugField(
        max_length=128, unique=True, validators=[validate_slug]
    )
    created_at: models.DateTimeField = models.DateTimeField(auto_now_add=True)
    updated_at: models.DateTimeField = models.DateTimeField(auto_now=True)

    members: models.ManyToManyField = models.ManyToManyField(
        "bench.User",
        through="OrganizationMembership",
        related_name="organizations",
        related_query_name="organization",
    )

    objects = OrganizationManager()

    def __str__(self):
        return self.slug


class OrganizationMembership(UUIDModel):
    # kept in sync with OrganizationMembership.Level
    class Level(models.IntegerChoices):
        Member = 1
        Administrator = 8
        Owner = 16

    organization: models.ForeignKey = models.ForeignKey(
        Organization, on_delete=models.CASCADE, related_name="memberships"
    )
    user: models.ForeignKey = models.ForeignKey(
        "bench.User", on_delete=models.CASCADE, related_name="organization_memberships+"
    )
    level: models.SmallIntegerField = models.SmallIntegerField(choices=Level.choices)

    created_at: models.DateTimeField = models.DateTimeField(auto_now_add=True)
    updated_at: models.DateTimeField = models.DateTimeField(auto_now=True)

    def __str__(self):
        return self.Level(self.level)

    class Meta:
        constraints = [
            models.UniqueConstraint(
                name="bench_organization_membership_ak", fields=["organization_id", "user_id"]
            )
        ]
