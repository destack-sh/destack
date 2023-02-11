from __future__ import annotations

from django.contrib.postgres.indexes import GinIndex
from django.contrib.postgres.search import SearchVector
from django.db import connection, models, transaction
from django.db.models import Count, F

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
                DatasetRecord(dataset=self, index=Count(F("records")) + 1, data=record)
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


class DatasetRecord(UUIDModel):
    """
    An individual JSON record. The data may be annotated with high level references.
    The references are resolved using dataset.annotations.
    """

    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    dataset = models.ForeignKey("Statement", on_delete=models.CASCADE, related_name="records")
    order_key = models.CharField(max_length=64)
    data = models.JSONField()

    def __str__(self):
        return f"{self.dataset}@{self.id.hex}[{self.order_key}]"

    class Meta:
        # order by index ascending by default
        ordering = ["order_key"]
        # TODO @Robustness: unique constraint on index when we switch to fractional indexes
        indexes = [
            GinIndex(SearchVector("data", config="simple"), name="bench_dataset_record_data"),
        ]
        constraints = [
            models.UniqueConstraint(
                fields=["dataset", "order_key"], name="bench_dataset_record_order_key_ak"
            ),
        ]
