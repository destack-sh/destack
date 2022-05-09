from __future__ import annotations

from typing import Optional

from django.db import models

from bench.models.utils import MAX_DESCRIPTION_LENGTH, MAX_NAME_LENGTH, UUIDModel
from bench.models.versioning import VersionedCommit, VersionedRepository


class ArtifactManager(models.Manager):
    def create_artifact(
        self, type: str, name: str, description: Optional[str]
    ) -> Artifact:
        artifact = Artifact(type=type, name=name, description=description)
        artifact.save()
        return artifact


class Artifact(UUIDModel, VersionedRepository):
    """
    A data artifact of any type with a unique name, produced by some process.

    Artifacts are versioned. Known versions are available in 'versions'. If we own
    the artifact (i.e. it is not externally controlled), 'versions' is exhaustive.

    Artifact data may be stored outside the DB (e.g., large models, images) and should be
    stored directly in the DB where appropriate/convenient (e.g., metadata, metrics).
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


class ArtifactVersion(UUIDModel, VersionedCommit):
    """
    An artifact version is a specific (generally) immutable state of an artifact.
    """

    artifact = models.ForeignKey(
        Artifact, on_delete=models.CASCADE, related_name="versions"
    )
    version = models.CharField(max_length=256)
    parents = models.ManyToManyField("ArtifactVersion", symmetrical=False)

    # snapshot data (may move into separate ArtifactSnapshot table at some point)
    record_tree = models.ForeignKey(
        "RecordTree", on_delete=models.RESTRICT, blank=True, null=True
    )
    storage_uri = models.CharField(max_length=512, blank=True, null=True)
    metadata = models.JSONField()

    class Meta:
        indexes = [
            models.Index(name="bench_artifact_version_version_idx", fields=["version"]),
        ]
        constraints = [
            models.UniqueConstraint(
                name="bench_artifact_version_artifact_version_ak",
                fields=["artifact", "version"],
            )
        ]


class ArtifactView(UUIDModel):
    """
    A pass-through (generally) immutable view of an Artifact. The data remains in the
    Artifact (or, rather, a specific version) and is only accessed through the view.

    If this view works only with specific versions, then it must specify the compatible
    versions in 'compatible_versions'. If empty, this view is assumed to be general.
    TODO @Feature: version artifact views
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
    data = models.JSONField()
