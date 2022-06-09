from __future__ import annotations

import secrets

from django.db import models

from bench.models.utils import MAX_DESCRIPTION_LENGTH, MAX_NAME_LENGTH, UUIDModel
from bench.models.versioning import VersionedCommit, VersionedRepository


class ArtifactManager(models.Manager):
    pass


class Artifact(UUIDModel, VersionedRepository):
    """
    A data artifact of any type with a unique name, produced by some process.

    Artifacts are versioned. Known versions are available in 'versions'. If we own
    the artifact (i.e. it is not externally controlled), 'versions' is exhaustive.

    Artifact data may be stored outside the DB (e.g., large models, images) and can be
    stored directly in the DB where appropriate/convenient (e.g., metadata, metrics).
    """

    type = models.CharField(max_length=64)
    name = models.CharField(max_length=MAX_NAME_LENGTH)
    description = models.CharField(max_length=MAX_DESCRIPTION_LENGTH, null=True, blank=True)
    created_at = models.DateTimeField(auto_now_add=True)
    controller = models.ForeignKey("Controller", on_delete=models.SET_NULL, blank=True, null=True)

    objects = ArtifactManager()

    @property
    def owned(self):
        return self.controller is None

    def __str__(self):
        return f"{self.type}:{self.name}"

    class Meta:
        indexes = [
            models.Index(name="bench_artifact_type_idx", fields=["type"]),
            models.Index(name="bench_artifact_name_idx", fields=["name"]),
        ]
        constraints = [models.UniqueConstraint(name="bench_artifact_name_ak", fields=["name"])]


def _generate_artifact_version(nbytes: int = 4) -> str:
    return secrets.token_hex(nbytes)


class ArtifactVersion(UUIDModel, VersionedCommit):
    """
    An artifact version is a specific (generally) immutable state of an artifact.

    If the version is owned by us, ArtifactVersion.version is a generated short unique id.
    If the version corresponds to an external artifact, the version may be set as needed.
    """

    artifact = models.ForeignKey(Artifact, on_delete=models.CASCADE, related_name="versions")
    # TODO @Cleanup @Architecture: remove/rename ArtifactVersion.version
    #  Having 'version' inside ArtifactVersion, which is a VersionedCommit,
    #  is confusing. We need to synchronize with external VCS and we need internal
    #  version identifiers, so we can't (?) just use ArtifactVersion.id only.
    #  Similar logic applies to FlowVersion.version.
    version = models.CharField(max_length=256, default=_generate_artifact_version)
    parents = models.ManyToManyField("ArtifactVersion", symmetrical=False)

    # snapshot data (may move into separate ArtifactSnapshot table/tree object at some point)
    record_tree_root = models.ForeignKey(
        "RecordTree", on_delete=models.RESTRICT, blank=True, null=True
    )
    storage_uri = models.CharField(max_length=512, blank=True, null=True)
    metadata = models.JSONField()

    def __str__(self):
        return f"{self.artifact.name}/{self.version}"

    class Meta:
        indexes = [
            models.Index(name="bench_artifact_version_idx", fields=["version"]),
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
    """

    type = models.CharField(max_length=64)
    artifact = models.ForeignKey(Artifact, on_delete=models.CASCADE, related_name="views")
    compatible_versions = models.ManyToManyField(ArtifactVersion, related_name="views")
    name = models.CharField(max_length=MAX_NAME_LENGTH)
    description = models.CharField(max_length=MAX_DESCRIPTION_LENGTH, null=True, blank=True)
    created_at = models.DateTimeField(auto_now_add=True)
    data = models.JSONField()

    def __str__(self):
        return f"{self.name}[{self.name}]"
