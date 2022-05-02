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
    A data artifact of any type with a unique name, produced by some process.

    Artifacts may be versioned. Known versions are available in 'versions'. If we own
    the artifact (i.e. it is not externally controlled), 'versions' is exhaustive.

    Artifact data should be stored outside the DB (e.g., large models), but may be
    stored directly in the DB where appropriate or convenient (e.g., metrics, logs).
    """

    type = models.CharField(max_length=64)
    name = models.CharField(max_length=MAX_NAME_LENGTH)
    description = models.CharField(
        max_length=MAX_DESCRIPTION_LENGTH, null=True, blank=True
    )
    created_at = models.DateTimeField(auto_now_add=True)
    controller = models.ForeignKey(
        "Controller", on_delete=models.SET_NULL, blank=True, null=True
    )

    objects = ArtifactManager()

    @property
    def owned(self):
        return self.controller is None

    class Meta:
        indexes = [
            models.Index(name="bench_artifact_type_idx", fields=["type"]),
            models.Index(name="bench_artifact_name_idx", fields=["name"]),
        ]
        constraints = [
            models.UniqueConstraint(name="bench_artifact_name_ak", fields=["name"])
        ]


class ArtifactVersion(UUIDModel):
    """
    An artifact version is a specific (generally) immutable snapshot of an artifact.
    """

    artifact = models.ForeignKey(
        Artifact, on_delete=models.CASCADE, related_name="versions"
    )
    version = models.CharField(max_length=256)
    created_at = models.DateTimeField(auto_now_add=True)

    name = models.CharField(max_length=MAX_NAME_LENGTH, blank=True, null=True)
    storage_uri = models.CharField(max_length=512, blank=True, null=True)
    metadata = models.JSONField()

    class Meta:
        indexes = [models.Index(name="bench_artifact_version_idx", fields=["version"])]
        constraints = [
            models.UniqueConstraint(
                name="bench_artifact_version_ak", fields=["artifact", "version"]
            )
        ]


class ArtifactView(UUIDModel):
    """
    A pass-through (generally) immutable view of an Artifact. The data remains in the
    Artifact (or, rather, a specific version) and is only accessed through the view.

    If this view works only with specific versions (e.g. because it has dataset indices),
    then it must specify the compatible versions in 'compatible_versions'.
    """

    type = models.CharField(max_length=64)
    artifact = models.ForeignKey(
        Artifact, on_delete=models.CASCADE, related_name="views"
    )
    compatible_versions = models.ManyToManyField(ArtifactVersion, related_name="views")
    name = models.CharField(max_length=MAX_NAME_LENGTH)
    description = models.CharField(
        max_length=MAX_DESCRIPTION_LENGTH, null=True, blank=True
    )
    created_at = models.DateTimeField(auto_now_add=True)
    metadata = models.JSONField()
