from django.contrib.postgres.indexes import GinIndex
from django.db import models
from django.db.models import Q

from bench.models.utils import UUIDModel
from bench.models.versioning import VersionedBlob, VersionedTree


class Record(UUIDModel, VersionedBlob):
    """
    An individual record of a dataset-like Artifact.

    Data represents the in-DB part of the record's data corresponding to the artifact's
    schema, while metadata is additional data derived from the data, added by a user
    or function or any other information not strictly part of the data.
    Both data and metadata may include pointers to the artifact storage.
    """

    data = models.JSONField()
    metadata = models.JSONField(null=True, blank=True)

    class Meta:
        indexes = [GinIndex(name="bench_record_metadata", fields=["metadata"])]


class RecordTree(UUIDModel, VersionedTree):
    """
    A tree referring to other records or subtrees.
    """

    records = models.ManyToManyField(
        Record, through="RecordTreeReference", through_fields=("tree", "record")
    )
    subtrees = models.ManyToManyField(
        "RecordTree", through="RecordTreeReference", through_fields=("tree", "subtree")
    )


class RecordTreeReference(UUIDModel):
    """
    A reference from a 'tree' to either a record or a subtree at a relative index.
    """

    tree = models.ForeignKey(
        RecordTree, on_delete=models.CASCADE, related_name="references"
    )
    index = models.IntegerField()
    record = models.ForeignKey(
        Record, on_delete=models.CASCADE, null=True, blank=True, related_name="+"
    )
    subtree = models.ForeignKey(
        RecordTree, on_delete=models.CASCADE, null=True, blank=True, related_name="+"
    )

    class Meta:
        ordering = ["tree", "index"]
        constraints = [
            models.UniqueConstraint(
                name="bench_record_tree_reference_index_ak", fields=["tree", "index"]
            ),
            models.CheckConstraint(
                name="bench_record_tree_reference_set",
                check=Q(record__isnull=True, subtree__isnull=False)
                | Q(record__isnull=False, subtree__isnull=True),
            ),
        ]
