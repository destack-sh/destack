import datasets
import hub
from django.db import models

from bench.function.base import Record, RecordBatch
from bench.models.utils import MAX_DESCRIPTION_LENGTH, MAX_NAME_LENGTH, UUIDModel


class DatasetManager(models.Manager):
    pass


class Dataset(UUIDModel):
    name = models.CharField(max_length=MAX_NAME_LENGTH)
    description = models.CharField(max_length=MAX_DESCRIPTION_LENGTH)
    spec = models.JSONField()

    def append(self, record: Record):
        raise NotImplementedError

    def update(self, index: int, record: Record):
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


class ActiveloopDataset(Dataset):
    path = models.CharField(max_length=MAX_NAME_LENGTH)

    _hub_dataset_unsafe: hub.Dataset

    @property
    def _dataset(self) -> hub.Dataset:
        if self._hub_dataset_unsafe is None:
            self._hub_dataset_unsafe = hub.dataset(self.path)
        return self._hub_dataset_unsafe

    @property
    def versions(self):
        return self._dataset.commits

    @property
    def branches(self):
        return self._dataset.branches

    @property
    def records(self) -> RecordBatch:
        return self._dataset


class HuggingFaceDataset(Dataset):
    path = models.CharField(max_length=200)

    _hf_dataset_unsafe: datasets.Dataset

    @property
    def _dataset(self) -> datasets.Dataset:
        if self._hf_dataset_unsafe is None:
            self._hf_dataset_unsafe = datasets.load_dataset(self.path, streaming=True)
        return self._hf_dataset_unsafe

    @property
    def records(self) -> RecordBatch:
        return self._dataset


class DatasetVersion(models.Model):
    dataset = models.ForeignKey("Dataset", on_delete=models.CASCADE)
    version = models.CharField(max_length=256, blank=True)
    branch = models.CharField(max_length=256, null=True, blank=True)

    @property
    def records(self) -> RecordBatch:
        return self.dataset.records

    class Meta:
        constraints = [
            models.UniqueConstraint(
                fields=["dataset", "version"], name="bench_unique_dataset_version"
            )
        ]


class DatasetSlice(models.Model):
    dataset = models.ForeignKey("Dataset", on_delete=models.CASCADE)
