from __future__ import annotations

from django.db import models

from bench.models.utils import MAX_DESCRIPTION_LENGTH, UUIDTModel


class ModelContentMixin:
    """
    Model content as in language.Model
    TODO @Cleanup: Statement shouldn't contain all raw language.Model fields via ModelContentMixin,
     as that seems like too much specific info compared to the other symbol contents. Model is weird currently.
    """

    external_name = models.CharField(max_length=128, null=True, blank=True)
    description = models.CharField(max_length=MAX_DESCRIPTION_LENGTH, null=True, blank=True)
    provider = models.CharField(max_length=64)
    default_settings = models.JSONField(default=dict)


class ModelOperation(models.TextChoices):
    COMPLETE = "complete"


class ModelInference(UUIDTModel):
    """
    A single output from a model inference for debugging and caching.
    """

    model = models.ForeignKey("Statement", on_delete=models.CASCADE, related_name="inferences")
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
