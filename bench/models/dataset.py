from typing import List, Union

from django.db import models

from bench.function.base import Record, RecordBatch
from bench.models.utils import MAX_DESCRIPTION_LENGTH, MAX_NAME_LENGTH, UUIDModel


class DatasetManager(models.Manager):
    pass


class Dataset(UUIDModel):
    name = models.CharField(max_length=MAX_NAME_LENGTH)
    description = models.CharField(max_length=MAX_DESCRIPTION_LENGTH)
    spec = models.JSONField()
    storage_uri = models.CharField(max_length=512)
    controller_id = models.CharField(max_length=256, null=True)

    # where is it stored?
    # how do we load/stream/edit it?
    # what does it look like (spec)?

    def append(self, record: Record):
        raise NotImplementedError

    def extend(self, records: Union[RecordBatch, List[Record]]):
        raise NotImplementedError

    def set(self, index: int, record: Record):
        raise NotImplementedError

    def clear(self):
        raise NotImplementedError

    @property
    def versions(self):
        raise NotImplementedError

    @property
    def branches(self):
        raise NotImplementedError

    @property
    def records(self) -> RecordBatch:
        raise NotImplementedError

    objects: DatasetManager = DatasetManager()


class DatasetSlice(UUIDModel):
    dataset = models.ForeignKey("Dataset", on_delete=models.CASCADE)
