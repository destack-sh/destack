from django.db import models

from bench.models.artifact import Artifact
from bench.models.utils import UUIDModel
from bench.utils.spec import DatasetSpec


class DatasetManager(models.Manager):
    pass


class Dataset(Artifact):
    """
    A dataset describes a set of records of arbitrary format, which may be versioned.
    Datasets include machine learning datasets, function inputs/outputs, lexicons.
    Handling, storage and management of the dataset may be delegated to external services.
    """

    handler_id = models.CharField(max_length=256)
    arguments = models.JSONField()
    record_spec = models.JSONField()

    objects: DatasetManager = DatasetManager()

    def save(self, *args, **kwargs) -> None:
        # expose model-specific metadata in generic Artifact metadata
        self.metadata = {
            **self.metadata,
            "handler_id": self.handler_id,
            "arguments": self.arguments,
            "record_spec": self.record_spec,
        }
        super().save(*args, **kwargs)

    @property
    def spec(self) -> DatasetSpec:
        return DatasetSpec(record_spec=self.record_spec)


class DatasetSlice(UUIDModel):
    """
    A dataset slice is a point-in-time view of a dataset with certain filters.
    """

    dataset = models.ForeignKey("Dataset", on_delete=models.CASCADE)
    dataset_artifact = models.ForeignKey("ArtifactVersion", on_delete=models.CASCADE)
    dataset_version = models.CharField(max_length=256, blank=True, null=True)
    dataset_arguments = models.JSONField()
