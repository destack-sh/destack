from __future__ import annotations

import dataclasses
from typing import Any, AsyncIterator, Iterable, Iterator, Optional, Sequence

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

    name = models.CharField(max_length=MAX_NAME_LENGTH)
    created_at = models.DateTimeField(auto_now_add=True)

    # records from DatasetRecord.dataset
    # annotations from DatasetAnnotation.dataset
    schema = models.JSONField(null=True, blank=True)
    length = models.IntegerField(default=0)

    objects = DatasetManager()

    def __str__(self):
        return f"{self.name}.data@{self.id.hex}"

    def search_records(
        self, search: DatasetSearch, limit: int, offset: int
    ) -> Sequence[DatasetRecord]:
        raise NotImplementedError

    def get(self, index: int) -> DatasetRecord:
        return DatasetRecord.objects.get(dataset=self, index=index)

    def get_slice(self, start: int, stop: Optional[int] = None) -> Iterable[DatasetRecord]:
        stop = stop if stop is not None else 0
        return DatasetRecord.objects.filter(dataset=self)[start:stop]

    def get_field(self, key: str) -> Iterable[Any]:
        return DatasetRecord.objects.filter(dataset=self).values_list("data__" + key, flat=True)

    def __iter__(self) -> Iterator[dict]:
        for record in DatasetRecord.objects.filter(dataset=self):
            yield record.data

    async def __aiter__(self) -> AsyncIterator[dict]:
        async for record in DatasetRecord.objects.filter(dataset=self):
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
            db_record = self.get(index)
            db_record.data = record
            db_record.save()

    def delete_(self, index: int):
        with transaction.atomic():
            db_record = self.get(index)
            db_record.delete()
            # update the index of all records indices after the deleted one
            with connection.cursor() as cursor:
                cursor.execute(
                    "update bench_datasetrecord"
                    " set index = index - 1"
                    " where dataset_id = %s and index > %s",
                    [self.id, index],
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


class DatasetRecord(UUIDModel):
    """
    An individual JSON record. The data may be annotated with high level references.
    The references are resolved using dataset.annotations.
    """

    dataset = models.ForeignKey("Dataset", on_delete=models.CASCADE, related_name="records")
    index = models.IntegerField()
    data = models.JSONField()

    def __str__(self):
        return f"{self.dataset}/{self.index}@{self.id.hex}"

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

    def __str__(self):
        return f"{self.dataset.name}@{self.dataset.id.hex}/{self.name}"
