from django.db import models

from bench.models.utils import UUIDModel


class Artifact(UUIDModel):
    type = models.CharField(max_length=256)
    created_at = models.DateTimeField(auto_now_add=True)
    storage_uri = models.CharField(max_length=512, null=True)
    external = models.BooleanField()


class ArtifactVersion(UUIDModel):
    artifact = models.ForeignKey(
        Artifact, on_delete=models.CASCADE, related_name="versions"
    )
    created_at = models.DateTimeField(auto_now_add=True)
    storage_uri = models.CharField(max_length=512, null=True)
