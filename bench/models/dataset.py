from __future__ import annotations

import dataclasses
from dataclasses import dataclass
from typing import Any, Iterator, Optional, Sequence

from asgiref.sync import sync_to_async
from django.contrib.postgres.indexes import GinIndex
from django.contrib.postgres.search import SearchVector
from django.db import connection, models, transaction

from bench.models.tag import TaggableMixin
from bench.models.utils import MAX_DESCRIPTION_LENGTH, MAX_NAME_LENGTH, UUIDModel


class DatasetManager(models.Manager):
    pass


@dataclasses.dataclass
class DatasetSearch:
    text_like: Optional[str] = None


class Dataset(TaggableMixin, UUIDModel):
    """
    A dataset of JSON records.

    Datasets include machine learning datasets, function inputs/outputs, lexicons.
    """

    type = models.CharField(max_length=64)
    name = models.CharField(max_length=MAX_NAME_LENGTH)
    description = models.CharField(max_length=MAX_DESCRIPTION_LENGTH, null=True, blank=True)
    created_at = models.DateTimeField(auto_now_add=True)

    # records from DatasetRecord.dataset
    schema = models.JSONField()
    length = models.IntegerField(default=0)

    organization = models.ForeignKey(
        "Organization", on_delete=models.CASCADE, related_name="datasets"
    )
    project = models.ForeignKey("Project", on_delete=models.CASCADE, related_name="datasets+")

    objects = DatasetManager()

    def __str__(self):
        return f"{self.organization.slug}/{self.project.slug}/datasets/{self.name}@{self.id}"

    def search_records(
        self, search: DatasetSearch, limit: int, offset: int
    ) -> Sequence[DatasetRecord]:
        raise NotImplementedError

    def get_record(self, index: int) -> DatasetRecord:
        return DatasetRecord.objects.get(dataset=self, index=index)

    def get_records_view(self, view_data: Optional[DatasetViewData]) -> list[DatasetRecord]:
        if view_data is None:
            return self.get_records_slice(0, len(self))
        else:
            # note that this only works with slice-based views
            return self.get_records_slice(view_data.apply(0), view_data.apply(len(self)))

    def get_records_slice(self, start: int, stop: Optional[int] = None) -> list[DatasetRecord]:
        stop = stop if stop is not None else 0
        return DatasetRecord.objects.filter(dataset=self)[start:stop]

    def get_records_data_field(self, key: str) -> list[Any]:
        return DatasetRecord.objects.filter(dataset=self).values_list("data__" + key, flat=True)

    def __iter__(self) -> Iterator[dict]:
        for record in DatasetRecord.objects.filter(dataset=self):
            yield record.data

    def append(self, record: dict) -> int:
        with transaction.atomic():
            DatasetRecord.objects.create(dataset=self, index=self.length, data=record)
            self.length += 1
            self.save()
        return self.length

    async def aappend(self, record: dict) -> int:
        return await sync_to_async(self.append)(record)

    def extend(self, records: list[dict]) -> tuple[int, int]:
        start_length = self.length
        db_records = []
        # create db_records with incrementing index
        with transaction.atomic():
            for record in records:
                db_records.append(DatasetRecord(dataset=self, index=self.length, data=record))
                self.length += 1
            DatasetRecord.objects.bulk_create(db_records)
            self.save()

        return start_length, self.length

    async def aextend(self, records: list[dict]) -> tuple[int, int]:
        return await sync_to_async(self.extend)(records)

    def update(self, index: int, record: dict):
        with transaction.atomic():
            db_record = self.get_record(index)
            db_record.data = record
            db_record.save()

    def delete_(self, index: int):
        with transaction.atomic():
            db_record = self.get_record(index)
            db_record.delete()
            # update the index of all records indices after the deleted one
            with connection.cursor() as cursor:
                cursor.execute(
                    "update bench_datasetrecord"
                    " set index = index - 1"
                    " where dataset_id = %s and index > %s",
                    [self.dataset.id, index],
                )
            self.length -= 1
            self.save()

    def clear(self):
        with transaction.atomic():
            DatasetRecord.objects.filter(dataset=self).delete()
            self.length = 0
            self.save()

    def __len__(self) -> int:
        return self.length

    class Meta:
        indexes = []
        constraints = []


class DatasetRecord(UUIDModel):
    """
    An individual immutable record of a dataset-like Artifact.
    """

    dataset = models.ForeignKey("Dataset", on_delete=models.CASCADE, related_name="records")
    index = models.IntegerField()
    data = models.JSONField()

    def is_committed(self) -> bool:
        return True

    class Meta:
        # order by index ascending by default
        ordering = ["index"]
        constraints = [
            models.UniqueConstraint(
                fields=["dataset", "index"], name="bench_record_dataset_index_ak"
            ),
        ]
        indexes = [
            GinIndex(SearchVector("data", config="simple"), name="bench_record_data"),
        ]


class DatasetView(TaggableMixin, UUIDModel):
    """
    A view of a Dataset.
    """

    type = models.CharField(max_length=64)
    dataset = models.ForeignKey(Dataset, on_delete=models.CASCADE, related_name="views")
    name = models.CharField(max_length=MAX_NAME_LENGTH)
    description = models.CharField(max_length=MAX_DESCRIPTION_LENGTH, null=True, blank=True)
    created_at = models.DateTimeField(auto_now_add=True)
    data = models.JSONField()

    def __str__(self):
        return f"{self.name}[{self.name}]"


# TODO @Feature: support more complex dataset views (e.g. filters)
@dataclass
class DatasetViewData:
    start: Optional[int] = None
    end: Optional[int] = None

    @property
    def asdict(self) -> dict:
        return dataclasses.asdict(self)

    @staticmethod
    def empty() -> DatasetViewData:
        return DatasetViewData.from_slice((0, 0))

    @staticmethod
    def from_slice(slice: tuple[int, int]) -> DatasetViewData:
        return DatasetViewData(start=slice[0], end=slice[1])

    def apply(self, index: int) -> int:
        if self.start is not None:
            index += self.start
        if self.end is not None and index >= self.end:
            return self.end
        return index
