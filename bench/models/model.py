from __future__ import annotations

import copy
import dataclasses
from dataclasses import dataclass
from functools import cached_property
from typing import Any, Dict, Optional

from django.db import transaction
from django.db.models import QuerySet
from rest_framework import serializers

from bench.models.artifact import Artifact, ArtifactManager, ArtifactVersion
from bench.models.utils import MODEL_TYPE
from bench.utils.serializer import RecordSpecSerializer
from bench.utils.spec import ModelType, RecordSpec


class ModelManager(ArtifactManager):
    def get_queryset(self) -> QuerySet[Model]:
        return super().get_queryset().filter(type__exact=MODEL_TYPE)

    def create_model(
        self,
        name: str,
        description: Optional[str],
        metadata: ModelMetadata,
    ) -> Model:
        """Creates the given model with an initial version"""
        model = Model(type=MODEL_TYPE, name=name, description=description)
        model.versions.create(metadata=metadata.to_dict())
        model.save()
        return model

    def create_model_version(self, name: str, metadata: ModelMetadata) -> ModelVersion:
        """Creates model version and corresponding model if it doesn't exist"""
        with transaction.atomic():
            model, _ = Model.objects.get_or_create(type=MODEL_TYPE, name=name)
            model_version = ModelVersion(artifact=model, metadata=metadata.to_dict())
            model_version.save()
        return model_version


class Model(Artifact):
    """
    A model is an artifact representing a machine learning model.
    Handling, storage and management of the model may be delegated to external services.

    The flexible framework of handlers, controllers and underlying artifacts lets us own or
    delegate parts of the model lifecycle to accommodate different workflows. For example,
    we can run pre-trained HuggingFace models, local custom PyTorch models, hosted LLM
    models, from any storages (e.g., S3, disk) or controllers (e.g., DVC, Mlflow).
    TODO @Feature: specify model affordances for different tasks, define common task/model specs
    """

    objects = ModelManager()

    class Meta:
        proxy = True


@dataclass(frozen=True)
class ModelMetadata:
    handler_id: str
    config_arguments: Dict[str, Any] = dataclasses.field(default_factory=dict)
    input_spec: RecordSpec = RecordSpec(name="", description="", type={})
    output_spec: RecordSpec = RecordSpec(name="", description="", type={})

    @staticmethod
    def from_dict(obj: dict) -> ModelMetadata:
        serializer = ModelMetadataSerializer(data=copy.deepcopy(obj))
        serializer.is_valid(raise_exception=True)
        return serializer.save()

    def to_dict(self) -> dict:
        return ModelMetadataSerializer(self).data


class ModelMetadataSerializer(serializers.Serializer):
    handler_id = serializers.CharField()
    config_arguments = serializers.JSONField()
    input_spec = RecordSpecSerializer(required=False)
    output_spec = RecordSpecSerializer(required=False)

    def create(self, validated_data):
        if "input_spec" in validated_data:
            validated_data["input_spec"] = RecordSpec(**validated_data["input_spec"])
        if "output_spec" in validated_data:
            validated_data["output_spec"] = RecordSpec(**validated_data["output_spec"])
        return ModelMetadata(**validated_data)


class ModelVersion(ArtifactVersion):
    """
    A Model-specific thin proxy of ArtifactVersion exposing typed attributes.
    """

    @cached_property
    def metadata_typed(self) -> ModelMetadata:
        return ModelMetadata.from_dict(self.metadata)

    @cached_property
    def handler_id(self) -> str:
        return self.metadata_typed.handler_id

    @cached_property
    def config_arguments(self):
        return self.metadata_typed.config_arguments

    @cached_property
    def input_spec(self):
        return self.metadata_typed.input_spec

    @cached_property
    def output_spec(self):
        return self.metadata_typed.output_spec

    @property
    def spec(self) -> ModelType:
        return ModelType(input_spec=self.input_spec, output_spec=self.output_spec)

    class Meta:
        proxy = True
