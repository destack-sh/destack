from django.db import models

from bench.models.utils import UUIDModel


class OrganizationManager(models.Manager):
    pass


class Organization(UUIDModel):
    name: models.CharField = models.CharField(max_length=64)
    created_at: models.DateTimeField = models.DateTimeField(auto_now_add=True)
    updated_at: models.DateTimeField = models.DateTimeField(auto_now=True)

    objects = OrganizationManager()
