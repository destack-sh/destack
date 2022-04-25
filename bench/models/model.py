from django.db import models

from bench.models.artifact import Artifact
from bench.models.utils import MAX_DESCRIPTION_LENGTH, MAX_NAME_LENGTH
from bench.utils.spec import ModelSpec


class ModelManager(models.Manager):
    pass


class Model(Artifact):
    """
    A model describes a specific machine learning model, which may have multiple versions.
    Handling, storage and management of the model may be delegated to external services.

    The flexible framework of handlers, storages and managers lets us run or delegate
    each portion of the model lifecycle to accommodate different workflows. For example,
    we can run pre-trained HuggingFace models, local custom PyTorch models, hosted LLM
    models, any combination of storages (e.g., S3, disk) or managers (e.g., DVC, Mlflow).
    """

    name = models.CharField(max_length=MAX_NAME_LENGTH)
    description = models.CharField(max_length=MAX_DESCRIPTION_LENGTH)

    handler_id = models.CharField(max_length=256)
    manager_id = models.CharField(max_length=256, null=True)

    arguments = models.JSONField()
    input_spec = models.JSONField()
    output_spec = models.JSONField()

    objects = ModelManager()

    @property
    def spec(self) -> ModelSpec:
        return ModelSpec(input_spec=self.input_spec, output_spec=self.output_spec)
