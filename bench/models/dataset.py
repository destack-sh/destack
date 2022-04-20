from django.db import models

from bench.models.utils import MAX_DESCRIPTION_LENGTH, MAX_NAME_LENGTH, UUIDModel
from bench.utils.spec import DatasetSpec


class DatasetManager(models.Manager):
    pass


class Dataset(UUIDModel):
    """
    A dataset describes a data artefact of an arbitrary format with optional versioning,
    including machine learning datasets, function inputs/outputs, lexicons, and more.
    Handling, storage and management of the dataset may be delegated to external services.
    """

    name = models.CharField(max_length=MAX_NAME_LENGTH)
    description = models.CharField(max_length=MAX_DESCRIPTION_LENGTH)
    created_at = models.DateTimeField(auto_now_add=True)

    handler_id = models.CharField(max_length=256)
    storage_uri = models.CharField(max_length=512, null=True)
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
