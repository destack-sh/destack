from django.db import models

from bench.models.artifact import Artifact
from bench.models.utils import MAX_DESCRIPTION_LENGTH, MAX_NAME_LENGTH, UUIDModel
from bench.utils.spec import DatasetSpec


class DatasetManager(models.Manager):
    pass


class Dataset(Artifact):
    """
    A dataset describes a set of records of arbitrary format, which may be versioned.
    Datasets include machine learning datasets, function inputs/outputs, lexicons.
    Handling, storage and management of the dataset may be delegated to external services.
    """

    name = models.CharField(max_length=MAX_NAME_LENGTH)
    description = models.CharField(max_length=MAX_DESCRIPTION_LENGTH)

    handler_id = models.CharField(max_length=256)
    manager_id = models.CharField(max_length=256, null=True)

    arguments = models.JSONField()
    record_spec = models.JSONField()

    objects: DatasetManager = DatasetManager()

    @property
    def spec(self) -> DatasetSpec:
        return DatasetSpec(record_spec=self.record_spec)


class DatasetSlice(UUIDModel):
    """
    A dataset slice is a point-in-time view of a dataset with certain filters.
    """

    dataset = models.ForeignKey("Dataset", on_delete=models.CASCADE)
    dataset_version = models.CharField(max_length=256, null=True)
    dataset_arguments = models.JSONField()
