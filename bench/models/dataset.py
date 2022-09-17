from __future__ import annotations

import copy
import dataclasses
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Dict, Optional

from django.db import transaction
from django.db.models import QuerySet
from rest_framework import serializers

from bench.models.artifact import Artifact, ArtifactManager, ArtifactVersion
from bench.models.utils import DATASET_TYPE
from bench.utils.serializer import FieldSpecSerializer
from bench.utils.spec import DatasetType, RecordSpec

if TYPE_CHECKING:
    from bench.models import Organization


class DatasetManager(ArtifactManager):
    def get_queryset(self) -> QuerySet[Dataset]:
        return super().get_queryset().filter(type__exact=DATASET_TYPE)

    def create_dataset_version(
        self, name: str, organization: Organization, metadata: DatasetMetadata
    ) -> DatasetVersion:
        """Creates dataset version and corresponding dataset if it doesn't exist"""
        with transaction.atomic():
            dataset, _ = Dataset.objects.get_or_create(
                type=DATASET_TYPE, organization=organization, name=name
            )
            return DatasetVersion.objects.create(artifact=dataset, metadata=metadata.to_dict())

    def get_or_create_dataset_version(
        self, name: str, version: str, organization: Organization, metadata: DatasetMetadata
    ) -> DatasetVersion:
        """Creates dataset version and corresponding dataset if it doesn't exist"""
        dataset, _ = Dataset.objects.get_or_create(
            type=DATASET_TYPE, organization=organization, name=name
        )
        dataset_version, _ = DatasetVersion.objects.select_related(
            "record_tree_root"
        ).get_or_create(
            artifact__name=name,
            organization=organization,
            version=version,
            defaults={"metadata": metadata.to_dict()},
        )
        return dataset_version


class Dataset(Artifact):
    """
    A dataset is an artifact representing a set of records of arbitrary format.

    Datasets include machine learning datasets, function inputs/outputs, lexicons.
    Handling, storage and management of the dataset may be delegated to external services.
    """

    objects = DatasetManager()

    class Meta:
        proxy = True


@dataclass(frozen=True)
class DatasetMetadata:
    handler_id: str
    config_arguments: Dict[str, Any] = dataclasses.field(default_factory=dict)
    record_spec: RecordSpec = RecordSpec(name="", description="", type={})
    metadata_spec: RecordSpec = RecordSpec(name="", description="", type={})

    @staticmethod
    def from_dict(obj: dict) -> DatasetMetadata:
        serializer = DatasetMetadataSerializer(data=copy.deepcopy(obj))
        serializer.is_valid(raise_exception=True)
        return serializer.save()

    def to_dict(self) -> dict:
        return DatasetMetadataSerializer(self).data

    @staticmethod
    def default_db():
        return DatasetMetadata(handler_id="bench.db")


class DatasetMetadataSerializer(serializers.Serializer):
    handler_id = serializers.CharField()
    config_arguments = serializers.JSONField()
    record_spec = FieldSpecSerializer(required=False)
    metadata_spec = FieldSpecSerializer(required=False)

    def create(self, validated_data):
        if "record_spec" in validated_data:
            validated_data["record_spec"] = RecordSpec(**validated_data["record_spec"])
        return DatasetMetadata(**validated_data)


class DatasetVersion(ArtifactVersion):
    """
    A Dataset-specific thin proxy of ArtifactVersion exposing typed attributes.
    """

    @property
    def metadata_typed(self) -> DatasetMetadata:
        return DatasetMetadata.from_dict(self.metadata)

    @property
    def handler_id(self) -> str:
        return self.metadata_typed.handler_id

    @handler_id.setter
    def handler_id(self, value: str):
        self.metadata["handler_id"] = value

    @property
    def config_arguments(self):
        return self.metadata_typed.config_arguments

    @property
    def record_spec(self):
        return self.metadata_typed.record_spec

    @property
    def spec(self) -> DatasetType:
        return DatasetType(record_spec=self.metadata_typed.record_spec)

    class Meta:
        proxy = True


# TODO @Feature: support more complex dataset views (e.g. filters)
@dataclass
class DatasetViewData:
    start: Optional[int] = None
    end: Optional[int] = None

    @property
    def asdict(self) -> dict:
        return dataclasses.asdict(self)

    @staticmethod
    def empty() -> DatasetViewData:
        return DatasetViewData.from_slice((0, 0))

    @staticmethod
    def from_slice(slice: tuple[int, int]) -> DatasetViewData:
        return DatasetViewData(start=slice[0], end=slice[1])

    def apply(self, index: int) -> int:
        if self.start is not None:
            index += self.start
        if self.end is not None and index >= self.end:
            return self.end
        return index
