from __future__ import annotations

from datetime import datetime

import pytz
from django.contrib.postgres.indexes import GinIndex
from django.db import models
from django.db.models import Q

from bench.models.utils import CrudModel, UUIDModel


class DatasetRecordManager(models.Manager["DatasetRecord"]):
    def get_queryset(self) -> models.QuerySet[DatasetRecord]:
        # soft-deleted statements are not returned by default
        return super().get_queryset().filter(deleted_at__isnull=True)


class DatasetRecord(UUIDModel, CrudModel):
    """
    An individual JSON record.
    """

    statement = models.ForeignKey("Statement", on_delete=models.CASCADE, related_name="records")
    order_key = models.CharField(max_length=64)
    data = models.JSONField(blank=True)

    def __str__(self):
        return f"{self.statement} record[{self.order_key}]"

    def __repr__(self):
        return f"<DatasetRecord {str(self)}>"

    def soft_delete(self):
        self.deleted_at = datetime.utcnow().replace(tzinfo=pytz.utc)

    def restore(self):
        self.deleted_at = None

    objects = DatasetRecordManager()

    class Meta:
        ordering = ["order_key"]
        default_manager_name = "objects"
        indexes = [models.Index(fields=["statement"]), GinIndex(fields=["data"])]
        constraints = [
            models.UniqueConstraint(
                fields=["statement", "order_key"],
                name="bench_statement_dataset_record_order_key_ak",
                condition=Q(deleted_at__isnull=True),
            ),
        ]
