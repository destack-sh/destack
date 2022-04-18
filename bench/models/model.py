from django.db import models

from bench.model.base import ModelHandler
from bench.models.utils import MAX_DESCRIPTION_LENGTH, MAX_NAME_LENGTH, UUIDModel


class ModelManager(models.Manager):
    pass


class Model(UUIDModel):
    name = models.CharField(max_length=MAX_NAME_LENGTH)
    description = models.CharField(max_length=MAX_DESCRIPTION_LENGTH)

    handler_id = models.CharField(max_length=256)
    storage_uri = models.CharField(max_length=512, null=True)
    manager_id = models.CharField(max_length=256, null=True)

    # possibilities:
    # HF transformers model, remote
    # HF transformers model, owned, unmanaged
    # HF inference API, owned, model
    # Spacy model, owned, managed by DVC
    # OpenAI api, remote,
    # Azure Cognitive Services, remote, managed

    # current concerns:
    # how can we provide inference?
    # what framework does it use?
    # who manages it (e.g. DVC, Mlflow)?
    # if not remotely executed, where is it stored (e.g. S3, disk)?
    # is it metered (e.g. commercial API)?
    # is it throttled/should it be throttled?
    # can we get gradients/embeddings/etc.?
    # future concerns:
    # is it trainable?

    arguments = models.JSONField()
    input_spec = models.JSONField()
    output_spec = models.JSONField()

    objects = ModelManager()

    @property
    def handle(self) -> ModelHandler:
        raise NotImplementedError
