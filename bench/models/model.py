from __future__ import annotations

from collections import OrderedDict
from uuid import UUID

from django.contrib.postgres.fields import ArrayField
from django.db import models

from bench.models.symbol import Symbol, SymbolContent, SymbolContentManager
from bench.models.utils import MAX_DESCRIPTION_LENGTH, UUIDModel, UUIDTModel


class ProviderKey(models.TextChoices):
    OPENAI = "openai"
    GOOSEAI = "gooseai"
    AI21 = "ai21"


class ModelManager(SymbolContentManager, models.Manager["Model"]):
    def get_queryset(self):
        # always select default settings
        return super().get_queryset().select_related("default_settings")


class Model(SymbolContent):
    """
    A model is a language model provided and stored elsewhere.

    Baseline models are typically provided externally and may be fine-tuned within a project.
    We will likely later provide our own compute for model tuning and inference.
    """

    external_name = models.CharField(max_length=128, null=True, blank=True)
    description = models.CharField(max_length=MAX_DESCRIPTION_LENGTH, null=True, blank=True)
    # TODO @Feature @Architecture: Model.baseline is really Symbol.extends
    baseline = models.ForeignKey(
        "Model", on_delete=models.CASCADE, null=True, related_name="derivatives"
    )
    provider = models.CharField(max_length=64, choices=ProviderKey.choices)
    default_settings = models.ForeignKey("ModelInferenceSettings", on_delete=models.CASCADE)

    def __str__(self):
        return f"{self.symbol_str}(provider={self.provider}/{self.external_name})"

    def deepcopy(self, to: Model, refs: dict[UUID, Symbol | SymbolContent]):
        super().deepcopy(to, refs)
        # copy default settings
        to.default_settings = self.default_settings
        to.default_settings.pk = None
        to.default_settings.save()

    objects = ModelManager()

    class Meta:
        default_manager_name = "objects"
        base_manager_name = "objects"


class ModelInferenceSettings(UUIDModel):
    """
    The settings to use when running inference with a language model.
    TODO @Cleanup: move inference settings elsewhere (out of table into dataclass?)
    """

    max_tokens = models.IntegerField(default=512)
    temperature = models.FloatField(default=0.7)
    top_p = models.FloatField(default=1.0)
    n = models.IntegerField(default=1)
    stop = ArrayField(models.CharField(max_length=128), null=True, default=list)
    echo = models.BooleanField(default=False)
    presence_penalty = models.FloatField(default=0.0)
    frequency_penalty = models.FloatField(default=0.0)
    logit_bias = models.JSONField(null=True, blank=True)
    logprobs = models.IntegerField(default=2)

    def as_dict(self, omit_empty: bool = True):
        fields = OrderedDict(
            [
                ("max_tokens", self.max_tokens),
                ("temperature", self.temperature),
                ("top_p", self.top_p),
                ("logprobs", self.logprobs),
                ("n", self.n),
                ("stop", self.stop),
                ("echo", self.echo),
                ("presence_penalty", self.presence_penalty),
                ("frequency_penalty", self.frequency_penalty),
                ("logit_bias", self.logit_bias),
            ]
        )
        if omit_empty:
            return {k: v for k, v in fields.items() if v is not None and v != []}
        return fields


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
