from __future__ import annotations

from dataclasses import dataclass
from functools import cached_property
from typing import Any, Dict

from django.db import models

from bench.models.artifact import Artifact, ArtifactManager, ArtifactVersion
from bench.models.utils import DATASET_TYPE, MAX_NAME_LENGTH, UUIDModel
from bench.utils.spec import DatasetSpec


class DatasetManager(ArtifactManager):
    def get_queryset(self) -> models.QuerySet[Dataset]:
        return super().get_queryset().filter(type__exact=DATASET_TYPE)


class Dataset(Artifact):
    """
    A dataset is an artifact representing a set of records of arbitrary format.

    Datasets include machine learning datasets, function inputs/outputs, lexicons.
    Handling, storage and management of the dataset may be delegated to external services.
    """

    datasets = DatasetManager()

    class Meta:
        proxy = True


@dataclass(frozen=True)
class DatasetMetadata:
    handler_id: str
    arguments: Dict[str, Any]
    record_spec: Dict[str, Any]

    class Config:
        allow_mutation = False
        frozen = True


class DatasetVersion(ArtifactVersion):
    """
    A Dataset-specific thin proxy of ArtifactVersion exposing typed attributes.
    """

    @cached_property
    def _metadata(self) -> DatasetMetadata:
        return DatasetMetadata(**self.metadata)

    @cached_property
    def handler_id(self) -> str:
        return self._metadata.handler_id

    @cached_property
    def arguments(self):
        return self._metadata.arguments

    @cached_property
    def record_spec(self):
        return self._metadata.record_spec

    @property
    def spec(self) -> DatasetSpec:
        return DatasetSpec(record_spec=self._metadata.record_spec)

    class Meta:
        proxy = True


class DatasetSlice(UUIDModel):
    """
    A dataset slice is a point-in-time view of a dataset with certain filters.
    """

    created_at = models.DateTimeField(auto_now_add=True)
    name = models.CharField(max_length=MAX_NAME_LENGTH)
    dataset = models.ForeignKey("ArtifactVersion", on_delete=models.CASCADE)
