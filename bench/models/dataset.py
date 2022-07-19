from __future__ import annotations

import dataclasses
from dataclasses import dataclass
from functools import cached_property
from typing import Any, Dict, Optional

from dataclasses_json import dataclass_json
from django.db import transaction
from django.db.models import QuerySet

from bench.models.artifact import Artifact, ArtifactManager, ArtifactVersion
from bench.models.utils import DATASET_TYPE
from bench.utils.spec import DatasetSpec, RecordSpec


class DatasetManager(ArtifactManager):
    def get_queryset(self) -> QuerySet[Dataset]:
        return super().get_queryset().filter(type__exact=DATASET_TYPE)

    def create_dataset_version(self, name: str, metadata: DatasetMetadata) -> DatasetVersion:
        """Creates dataset version and corresponding dataset if it doesn't exist"""
        with transaction.atomic():
            dataset, _ = Dataset.objects.get_or_create(type=DATASET_TYPE, name=name)
            dataset_version = DatasetVersion(artifact=dataset, metadata=metadata.to_dict())  # type: ignore
            dataset_version.save()
        return dataset_version


class Dataset(Artifact):
    """
    A dataset is an artifact representing a set of records of arbitrary format.

    Datasets include machine learning datasets, function inputs/outputs, lexicons.
    Handling, storage and management of the dataset may be delegated to external services.
    """

    objects = DatasetManager()  # type: ignore

    class Meta:
        proxy = True


@dataclass_json
@dataclass(frozen=True)
class DatasetMetadata:
    handler_id: str
    config_arguments: Dict[str, Any] = dataclasses.field(default_factory=dict)
    record_spec: RecordSpec = RecordSpec(name="", description="", type={})

    @staticmethod
    def default_db():
        return DatasetMetadata(handler_id="bench.db")

    class Config:
        allow_mutation = False
        frozen = True


class DatasetVersion(ArtifactVersion):
    """
    A Dataset-specific thin proxy of ArtifactVersion exposing typed attributes.
    """

    @cached_property
    def _metadata_typed(self) -> DatasetMetadata:
        return DatasetMetadata.from_dict(self.metadata)  # type: ignore

    @cached_property
    def handler_id(self) -> str:
        return self._metadata_typed.handler_id

    @cached_property
    def config_arguments(self):
        return self._metadata_typed.config_arguments

    @cached_property
    def record_spec(self):
        return self._metadata_typed.record_spec

    @property
    def spec(self) -> DatasetSpec:
        return DatasetSpec(
            name=self.artifact.name,
            description=self.artifact.description or "",
            record_spec=self._metadata_typed.record_spec,
        )

    class Meta:
        proxy = True


@dataclass
class DatasetViewData:
    start: Optional[int]
    end: Optional[int]

    # TODO @Feature: support more complex dataset views (e.g. filters)
    def apply(self, index: int) -> Optional[int]:
        if self.start is not None:
            index += self.start
        if self.end is not None and index >= self.end:
            return None
        return index
