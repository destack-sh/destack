from typing import List

from django.db import models

from bench.model.base import ModelBase
from bench.models.utils import MAX_DESCRIPTION_LENGTH, MAX_NAME_LENGTH, UUIDModel


class ModelManager(models.Manager):
    pass


class Model(UUIDModel):
    name = models.CharField(max_length=MAX_NAME_LENGTH)
    description = models.CharField(max_length=MAX_DESCRIPTION_LENGTH)

    framework_id = models.CharField(max_length=256)
    storage_id = models.CharField(max_length=256)

    # possibilities:
    # HF transformers model, remote
    # HF transformers model, owned, unmanaged
    # Spacy model, owned, managed by DVC
    # OpenAI api, remote,

    # current concerns:
    # what framework does it use?
    # who manages it (e.g. DVC, Mlflow)?
    # if not remotely managed, where is it stored (e.g. S3, disk)?
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
    def handle(self) -> ModelBase:
        raise NotImplementedError

    def versions(self) -> List[str]:
        raise NotImplementedError
