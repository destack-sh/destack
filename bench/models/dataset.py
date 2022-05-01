from __future__ import annotations

from dataclasses import dataclass
from functools import cached_property
from typing import Any, Dict

from django.db.models import QuerySet

from bench.models.artifact import (
    Artifact,
    ArtifactManager,
    ArtifactVersion,
    ArtifactView,
)
from bench.models.utils import DATASET_TYPE
from bench.utils.spec import DatasetSpec, RecordSpec


class DatasetManager(ArtifactManager):
    def get_queryset(self) -> QuerySet[Dataset]:
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
    config_arguments: Dict[str, Any]
    record_spec: RecordSpec

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
    def config_arguments(self):
        return self._metadata.config_arguments

    @cached_property
    def record_spec(self):
        return self._metadata.record_spec

    @property
    def spec(self) -> DatasetSpec:
        return DatasetSpec(
            name=self.artifact.name,
            description=self.artifact.description or "",
            record_spec=self._metadata.record_spec,
        )

    class Meta:
        proxy = True


class DatasetView(ArtifactView):
    """
    A Dataset-specific thin proxy of ArtifactView exposing typed attributes.
    """

    class Meta:
        proxy = True
