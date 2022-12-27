from __future__ import annotations

import dataclasses
from typing import Any, AsyncIterator, Iterable, Iterator, Optional, Sequence, cast
from uuid import UUID

from django.contrib.postgres.indexes import GinIndex
from django.contrib.postgres.search import SearchVector
from django.db import connection, models, transaction

from bench.models.schema_field import Schemad
from bench.models.symbol import Statement, SymbolContent, SymbolContentManager
from bench.models.utils import UUIDModel
from bench.utils.schema import derive_schema_from_records


class DatasetManager(SymbolContentManager, models.Manager["Dataset"]):
    def from_list(self, records: list[dict]) -> Dataset:
        dataset = cast(Dataset, self.create())
        dataset.set(records)
        return dataset


@dataclasses.dataclass
class DatasetSearch:
    text_like: Optional[str] = None


class Dataset(Schemad, SymbolContent):
    """
    A dataset of JSON records.
    """

    records: models.QuerySet["DatasetRecord"]  # noqa via DatasetRecord.dataset
    length = models.IntegerField(default=0)

    objects: DatasetManager = DatasetManager()

    def deepcopy(self, to: Dataset, refs: dict[UUID, Statement | SymbolContent]):
        super().deepcopy(to, refs)
        # copy nested non-symbol relations
        to.set(list(self))

    def __str__(self):
        return f"({self.length}*{self.schema or '<no schema>'})"

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
        for record in self.records.all():
            yield record.data

    async def __aiter__(self) -> AsyncIterator[dict]:
        async for record in self.records.all():
            yield record.data

    def append(self, record: dict) -> int:
        with transaction.atomic():
            self.records.create(index=self.length, data=record)
            self.length += 1
            self.save()
        return self.length

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

    @transaction.atomic
    def set(self, records: list[dict]):
        self.clear()
        self.extend(records)

    @transaction.atomic
    def derive_schema(self):
        """Derives and sets a new schema from the records."""
        records = list(self)
        new_schema_element = derive_schema_from_records(records)
        self.set_schema_element(new_schema_element)

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
            # when we finally switch to fractional indices, this will be easier and faster
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
        return f"{self.dataset}@{self.id.hex}[{self.index}]"

    class Meta:
        # order by index ascending by default
        ordering = ["index"]
        # TODO @Robustness: unique constraint on index when we switch to fractional indexes
        indexes = [
            GinIndex(SearchVector("data", config="simple"), name="bench_record_data"),
        ]


class Value(Schemad, SymbolContent):
    """A single JSON-ish value, akin to a dataset record but as an explicit statement."""

    value = models.JSONField(null=True, blank=True)
