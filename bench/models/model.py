from __future__ import annotations

from dataclasses import dataclass
from functools import cached_property
from typing import Any, Dict, Optional

from dataclasses_json import dataclass_json
from django.db import transaction
from django.db.models import QuerySet

from bench.models.artifact import Artifact, ArtifactManager, ArtifactVersion
from bench.models.utils import MODEL_TYPE
from bench.utils.spec import ModelSpec, RecordSpec


class ModelManager(ArtifactManager):
    def get_queryset(self) -> QuerySet[Model]:
        return super().get_queryset().filter(type__exact=MODEL_TYPE)

    def create_model(
        self,
        name: str,
        description: Optional[str],
        initial_version: str,
        metadata: ModelMetadata,
    ) -> Model:
        """Creates the given model with an initial version"""
        model = Model(type=MODEL_TYPE, name=name, description=description)
        model.versions.create(version=initial_version, metadata=metadata.to_dict())
        model.save()
        return model

    def create_model_version_by_name(
        self, name: str, version: str, metadata: ModelMetadata
    ) -> ModelVersion:
        """Creates model version and corresponding model if it doesn't exist"""
        with transaction.atomic():
            model, _ = Model.objects.get_or_create(type=MODEL_TYPE, name=name)
            model_version = ModelVersion(
                artifact=model, version=version, metadata=metadata.to_dict()
            )
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

    models = ModelManager()

    class Meta:
        proxy = True


@dataclass_json
@dataclass(frozen=True)
class ModelMetadata:
    handler_id: str
    config_arguments: Dict[str, Any]
    input_spec: RecordSpec
    output_spec: RecordSpec


class ModelVersion(ArtifactVersion):
    """
    A Model-specific thin proxy of ArtifactVersion exposing typed attributes.
    """

    @cached_property
    def _metadata_typed(self) -> ModelMetadata:
        return ModelMetadata.from_dict(self.metadata)

    @cached_property
    def handler_id(self) -> str:
        return self._metadata_typed.handler_id

    @cached_property
    def config_arguments(self):
        return self._metadata_typed.config_arguments

    @cached_property
    def input_spec(self):
        return self._metadata_typed.input_spec

    @cached_property
    def output_spec(self):
        return self._metadata_typed.input_spec

    @property
    def spec(self) -> ModelSpec:
        return ModelSpec(
            name=self.artifact.name,
            description=self.artifact.description or "",
            input_spec=self.input_spec,
            output_spec=self.output_spec,
        )

    class Meta:
        proxy = True
