from django.db import models

from bench.models.utils import UUIDModel


class TeamManager(models.Manager):
    pass


class Team(UUIDModel):
    name: models.CharField = models.CharField(max_length=64)
    created_at: models.DateTimeField = models.DateTimeField(auto_now_add=True)
    updated_at: models.DateTimeField = models.DateTimeField(auto_now=True)

    organization: models.ForeignKey = models.ForeignKey(
        "Organization", on_delete=models.CASCADE, related_name="teams"
    )

    objects = TeamManager()
