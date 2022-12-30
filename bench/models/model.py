from __future__ import annotations

from django.db import models

from bench.models.symbol import SymbolContent, SymbolContentManager
from bench.models.utils import MAX_DESCRIPTION_LENGTH, UUIDTModel


class ModelManager(SymbolContentManager, models.Manager["Model"]):
    pass


class Model(SymbolContent):
    """
    A model is a language model provided and stored elsewhere.

    Baseline models are typically provided externally and may be fine-tuned within a project.
    We will likely later provide our own compute for model tuning and inference.
    """

    external_name = models.CharField(max_length=128, null=True, blank=True)
    description = models.CharField(max_length=MAX_DESCRIPTION_LENGTH, null=True, blank=True)
    provider = models.CharField(max_length=64)
    default_settings = models.JSONField(default=dict)

    def __str__(self):
        return f"(provider={self.provider}/{self.external_name})"

    objects = ModelManager()

    class Meta:
        default_manager_name = "objects"
        base_manager_name = "objects"


class ModelOperation(models.TextChoices):
    COMPLETE = "complete"


class ModelInference(UUIDTModel):
    """
    A single output from a model inference for debugging and caching.
    """

    model = models.ForeignKey("Model", on_delete=models.CASCADE, related_name="inferences")
    operation = models.CharField(max_length=64, choices=ModelOperation.choices)
    settings_hash = models.CharField(max_length=64)
    settings = models.JSONField()
    input_hash = models.CharField(max_length=64)
    input = models.JSONField(null=True)
    output = models.JSONField()
    duration_ms = models.IntegerField()

    def __str__(self):
        return f"{self.model}/{self.id.hex}.inference(operation={self.operation})"

    class Meta:
        indexes = [
            models.Index(fields=["model", "operation", "settings_hash", "input_hash"]),
        ]
