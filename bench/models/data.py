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

    dataset = models.ForeignKey("Statement", on_delete=models.CASCADE, related_name="records")
    index = models.IntegerField()
    order_key = models.CharField(max_length=32, null=True, blank=True)  # in file/parent
    data = models.JSONField()

    def __str__(self):
        return f"{self.dataset}@{self.id.hex}[{self.index}]"

    class Meta:
        # order by index ascending by default
        ordering = ["index"]
        # TODO @Robustness: unique constraint on index when we switch to fractional indexes
        indexes = [
            GinIndex(SearchVector("data", config="simple"), name="bench_record_data"),
        ]
