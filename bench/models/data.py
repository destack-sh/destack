from __future__ import annotations

from datetime import datetime

import pytz
from django.db import connection, models, transaction
from django.db.models import Count, F, Q

from bench.models.utils import UUIDModel


class DatasetContentMixin:
    """Dataset content of JSON records."""

    records: models.QuerySet["DatasetRecord"]  # noqa via DatasetRecord.dataset

    def get_record(self, index: int) -> DatasetRecord:
        return DatasetRecord.objects.get(dataset=self, index=index)

    def append_record(self, record: dict):
        self.records.create(index=Count(F("records")), data=record)

    def extend_records(self, records: list[dict]):
        db_records = []
        # create db_records with incrementing index
        for i, record in enumerate(records):
            db_records.append(
                DatasetRecord(statement=self, index=Count(F("records")) + 1, data=record)
            )
        DatasetRecord.objects.bulk_create(db_records)

    @transaction.atomic
    def set_records(self, records: list[dict]):
        self.clear_records()
        self.extend_records(records)

    def update_record(self, index: int, record: dict):
        with transaction.atomic():
            db_record = self.get_record(index)
            db_record.data = record
            db_record.save()

    def delete_record(self, index: int):
        with transaction.atomic():
            db_record = self.get_record(index)
            db_record.delete()
            # update the index of all records indices after the deleted one
            # when we finally switch to fractional indices, this will be easier and faster
            with connection.cursor() as cursor:
                cursor.execute(
                    "update bench_datasetrecord"
                    " set index = index - 1"
                    " where dataset_id = %s and index > %s",
                    [self.id, index],
                )

    def clear_records(self):
        DatasetRecord.objects.filter(dataset=self).delete()


class DatasetRecordManager(models.Manager["DatasetRecord"]):
    def get_queryset(self) -> models.QuerySet[DatasetRecord]:
        # soft-deleted statements are not returned by default
        return super().get_queryset().filter(deleted_at__isnull=True)


class DatasetRecord(UUIDModel):
    """
    An individual JSON record.
    """

    statement = models.ForeignKey("Statement", on_delete=models.CASCADE, related_name="records")
    revision = models.IntegerField(default=1)
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    deleted_at = models.DateTimeField(null=True, blank=True)
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
        indexes = [models.Index(fields=["statement"])]
        constraints = [
            models.UniqueConstraint(
                fields=["statement", "order_key"],
                name="bench_statement_dataset_record_order_key_ak",
                condition=Q(deleted_at__isnull=True),
            ),
        ]
