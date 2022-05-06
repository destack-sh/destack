from django.db import models

from bench.models.utils import UUIDModel
from bench.models.versioning import VersionedBlob


class Record(UUIDModel, VersionedBlob):
    """
    An individual record of a dataset-like Artifact.

    Data represents the in-DB part of the record's data corresponding to the artifact's
    schema, while metadata is additional data derived from the data, added by a user
    or function or any other information not strictly part of the data.
    Both data and metadata may include pointers to the artifact storage.
    """

    artifact = models.ForeignKey("ArtifactVersion", on_delete=models.CASCADE)
    data = models.JSONField()
    metadata = models.JSONField(null=True, blank=True)
