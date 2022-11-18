from __future__ import annotations

from collections import OrderedDict

from django.contrib.postgres.fields import ArrayField
from django.db import models
from rest_framework import serializers

from bench.models.tag import TaggableMixin
from bench.models.utils import MAX_DESCRIPTION_LENGTH, MAX_NAME_LENGTH, UUIDModel


class ModelManager(models.Manager):
    pass


class ModelType(models.TextChoices):
    LLM = "llm"
    LLM_TUNED = "llm_tuned"


class ProviderKey(models.TextChoices):
    OPENAI = "openai"


class Model(TaggableMixin, UUIDModel):
    """
    A model is a language model provided and stored elsewhere.

    Baseline models are typically provided externally and may be fine-tuned within a project.
    We will likely later provide our own compute for model tuning and inference.
    """

    type = models.CharField(max_length=64, choices=ModelType.choices)
    name = models.CharField(max_length=MAX_NAME_LENGTH)
    description = models.CharField(max_length=MAX_DESCRIPTION_LENGTH, null=True, blank=True)
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)

    parent = models.ForeignKey(
        "Model", on_delete=models.CASCADE, null=True, related_name="children"
    )
    provider = models.CharField(max_length=64, choices=ProviderKey.choices)
    default_settings = models.ForeignKey("ModelInferenceSettings", on_delete=models.CASCADE)

    objects = ModelManager()

    def __str__(self):
        return f"{self.name}.{self.type}@{self.id.hex}"


class ModelInferenceSettings(UUIDModel):
    """
    The settings to use when running inference with a language model.
    """

    max_tokens = models.IntegerField(default=512)
    temperature = models.FloatField(default=0.7)
    top_p = models.FloatField(default=1.0)
    logprobs = models.IntegerField(default=2)
    n = models.IntegerField(default=1)
    stop = ArrayField(models.CharField(max_length=128), default=list)
    echo = models.BooleanField(default=False)
    tfs = models.FloatField(null=True, blank=True)
    presence_penalty = models.FloatField(default=0.0)
    frequency_penalty = models.FloatField(default=0.0)
    logit_bias = models.JSONField(null=True, blank=True)


class ModelInferenceSettingsSerializer(serializers.ModelSerializer):
    # exclude empty values from output
    def to_representation(self, value: ModelInferenceSettings) -> OrderedDict:
        repr_dict = super(serializers.ModelSerializer, self).to_representation(value)
        excluded_values = [None, [], "", {}]
        return OrderedDict((k, v) for k, v in repr_dict.items() if v not in excluded_values)

    class Meta:
        model = ModelInferenceSettings
        fields = "__all__"
