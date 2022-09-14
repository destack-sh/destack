from django.db import models

from bench.models.utils import UUIDModel


class OrganizationManager(models.Manager):
    pass


class Organization(UUIDModel):
    name: models.CharField = models.CharField(max_length=256)
    slug: models.CharField = models.CharField(max_length=128, unique=True)
    created_at: models.DateTimeField = models.DateTimeField(auto_now_add=True)
    updated_at: models.DateTimeField = models.DateTimeField(auto_now=True)

    members: models.ManyToManyField = models.ManyToManyField(
        "bench.User", through="OrganizationMembership", related_name="organizations"
    )

    objects = OrganizationManager()


class OrganizationMembership(UUIDModel):
    organization: models.ForeignKey = models.ForeignKey(
        Organization, on_delete=models.CASCADE, related_name="memberships"
    )
    user: models.ForeignKey = models.ForeignKey(
        "bench.User", on_delete=models.CASCADE, related_name="organization_memberships+"
    )

    created_at: models.DateTimeField = models.DateTimeField(auto_now_add=True)
    updated_at: models.DateTimeField = models.DateTimeField(auto_now=True)
