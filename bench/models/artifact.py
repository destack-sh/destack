from django.db import models

from bench.models.utils import MAX_DESCRIPTION_LENGTH, MAX_NAME_LENGTH, UUIDModel


class Artifact(UUIDModel):
    """
    A data artifact of any type produced by our own or an external process.
    Artifacts may be versioned, and if we own it, versions are available in 'versions'.
    Artifact data should be stored outside the DB (e.g., large models), but may be
    stored directly in the DB where that is more convenient (e.g., metrics).
    """

    type = models.CharField(max_length=256)
    name = models.CharField(max_length=MAX_NAME_LENGTH)
    description = models.CharField(max_length=MAX_DESCRIPTION_LENGTH, null=True)
    created_at = models.DateTimeField(auto_now_add=True)

    storage_uri = models.CharField(max_length=512, null=True)


class ArtifactVersion(models.Model):
    """
    An artifact version defines a specific immutable revision of an artifact.
    """

    artifact = models.ForeignKey(
        Artifact, on_delete=models.CASCADE, related_name="versions"
    )
    version = models.CharField(max_length=256)

    name = models.CharField(max_length=MAX_NAME_LENGTH, null=True)
    storage_uri = models.CharField(max_length=512, null=True)
    created_at = models.DateTimeField(auto_now_add=True)

    class Meta:
        constraints = [
            models.UniqueConstraint(
                name="bench_artifact_versions_pk", fields=["artifact", "version"]
            )
        ]
