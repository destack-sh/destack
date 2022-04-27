from __future__ import annotations

from typing import Optional

from django.db import models

from bench.models.utils import MAX_DESCRIPTION_LENGTH, MAX_NAME_LENGTH, UUIDModel


class ArtifactManager(models.Manager):
    def create_artifact(
        self, type: str, name: str, description: Optional[str]
    ) -> Artifact:
        artifact = Artifact(type=type, name=name, description=description)
        artifact.save()
        return artifact


class Artifact(UUIDModel):
    """
    A data artifact of any type produced by an owned or an external process.

    Artifacts may be versioned. Known versions are available in 'versions'. If we own
    the artifact (i.e. it is not externally controlled), 'versions' is exhaustive.

    Artifact data should be stored outside the DB (e.g., large models), but may be
    stored directly in the DB where appropriate or convenient (e.g., metrics, logs).
    """

    type = models.CharField(max_length=64)
    name = models.CharField(max_length=MAX_NAME_LENGTH)
    description = models.CharField(max_length=MAX_DESCRIPTION_LENGTH, blank=True)
    created_at = models.DateTimeField(auto_now_add=True)
    controller = models.ForeignKey(
        "Controller", on_delete=models.SET_NULL, blank=True, null=True
    )

    objects = ArtifactManager()

    @property
    def owned(self):
        return self.controller is None


class ArtifactVersionManager(models.Manager):
    pass


class ArtifactVersion(UUIDModel):
    """
    An artifact version defines a specific immutable snapshot of an artifact.
    """

    artifact = models.ForeignKey(
        Artifact, on_delete=models.CASCADE, related_name="versions"
    )
    version = models.CharField(max_length=256)
    created_at = models.DateTimeField(auto_now_add=True)

    name = models.CharField(max_length=MAX_NAME_LENGTH, blank=True, null=True)
    storage_uri = models.CharField(max_length=512, blank=True, null=True)
    metadata = models.JSONField()

    objects = ArtifactVersionManager()

    class Meta:
        constraints = [
            models.UniqueConstraint(
                name="bench_artifact_version_ak", fields=["artifact", "version"]
            )
        ]
