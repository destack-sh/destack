from __future__ import annotations

from dataclasses import dataclass
from functools import cached_property
from typing import Any, Dict

from django.db.models import QuerySet

from bench.models.artifact import Artifact, ArtifactManager, ArtifactVersion
from bench.models.utils import MODEL_TYPE
from bench.utils.spec import ModelSpec, RecordSpec


class ModelManager(ArtifactManager):
    def get_queryset(self) -> QuerySet[Model]:
        return super().get_queryset().filter(type__exact=MODEL_TYPE)


class Model(Artifact):
    """
    A model is an artifact representing a machine learning model.
    Handling, storage and management of the model may be delegated to external services.

    The flexible framework of handlers, managers and underlying artifacts lets us own or
    delegate parts of the model lifecycle to accommodate different workflows. For example,
    we can run pre-trained HuggingFace models, local custom PyTorch models, hosted LLM
    models, from any combination of storages (e.g., S3, disk) or managers (e.g., DVC, Mlflow).
    """

    models = ModelManager()

    class Meta:
        proxy = True


@dataclass(frozen=True)
class ModelMetadata:
    handler_id: str
    arguments: Dict[str, Any]
    input_spec: RecordSpec
    output_spec: RecordSpec


class ModelVersion(ArtifactVersion):
    """
    A Model-specific thin proxy of ArtifactVersion exposing typed attributes.
    """

    @cached_property
    def _metadata(self) -> ModelMetadata:
        return ModelMetadata(**self.metadata)

    @cached_property
    def handler_id(self) -> str:
        return self._metadata.handler_id

    @cached_property
    def arguments(self):
        return self._metadata.arguments

    @cached_property
    def input_spec(self):
        return self._metadata.input_spec

    @cached_property
    def output_spec(self):
        return self._metadata.input_spec

    @property
    def spec(self) -> ModelSpec:
        return ModelSpec(
            name=self.artifact.name,
            description=self.artifact.description,
            input_spec=self.input_spec,
            output_spec=self.output_spec,
        )

    class Meta:
        proxy = True
