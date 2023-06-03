from __future__ import annotations

from datetime import datetime

import pytz
from django.db import models
from django.db.models import Q

from bench.models.utils import CrudModel, UUIDModel

# TODO @Incomplete: actually use this dataset model


class DatasetBackend(models.TextChoices):
    """The backend used to store the dataset."""

    DB = "DB"
    OPENSEARCH = "OPENSEARCH"


class Dataset(UUIDModel):
    """A user created dataset backing the data symbol of a statement."""

    statement = models.OneToOneField("Statement", on_delete=models.CASCADE, related_name="dataset")
    backend = models.CharField(max_length=64, choices=DatasetBackend.choices)
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    versioned = models.BooleanField(default=True)
    os_index_name = models.CharField(max_length=256, null=True)
    os_pending_task_id = models.CharField(max_length=256, null=True)


class OpensearchMapping(models.Model):
    """An OpenSearch field mapping for a dataset."""

    id = models.IntegerField(primary_key=True)
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    deleted_at = models.DateTimeField(null=True)
    dataset = models.ForeignKey("Dataset", on_delete=models.CASCADE, related_name="os_mappings")
    mapping = models.JSONField()
    field = models.ForeignKey("Field", on_delete=models.SET_NULL, related_name="+", null=True)


class RecordManager(models.Manager["Record"]):
    def get_queryset(self) -> models.QuerySet[Record]:
        # soft-deleted statements are not returned by default
        return super().get_queryset().filter(deleted_at__isnull=True)


class Record(UUIDModel, CrudModel):
    """An individual JSON record."""

    statement = models.ForeignKey("Statement", on_delete=models.CASCADE, related_name="records")
    order_key = models.CharField(max_length=64)
    data = models.JSONField(blank=True)

    def __str__(self):
        return f"{self.statement} record[{self.order_key}]"

    def __repr__(self):
        return f"<Record {str(self)}>"

    def soft_delete(self):
        self.deleted_at = datetime.utcnow().replace(tzinfo=pytz.utc)

    def restore(self):
        self.deleted_at = None

    objects = RecordManager()

    class Meta:
        ordering = ["order_key"]
        default_manager_name = "objects"
        indexes = [models.Index(fields=["statement"])]
        constraints = [
            models.UniqueConstraint(
                fields=["statement", "order_key"],
                name="bench_statement_dataset_record_order_key_ak",
                condition=Q(deleted_at__isnull=True),
            ),
        ]
