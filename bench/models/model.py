from __future__ import annotations

from django.db import models

from bench.models.tag import TaggableMixin
from bench.models.utils import MAX_DESCRIPTION_LENGTH, MAX_NAME_LENGTH, UUIDModel


class ModelManager(models.Manager):
    pass


class ModelType(models.TextChoices):
    LLM = "llm"
    LLM_TUNED = "llm_tuned"


class Model(TaggableMixin, UUIDModel):
    """
    A model is a language model provided and stored elsewhere.

    Baseline models are typically provided externally and may then be fine-tuned within a project.
    """

    type = models.CharField(max_length=64, choices=ModelType.choices)
    name = models.CharField(max_length=MAX_NAME_LENGTH)
    description = models.CharField(max_length=MAX_DESCRIPTION_LENGTH, null=True, blank=True)
    created_at = models.DateTimeField(auto_now_add=True)
    handler_id = models.CharField(max_length=64)

    objects = ModelManager()

    def __str__(self):
        return f"{self.name}.{self.type}@{self.id.hex}"
