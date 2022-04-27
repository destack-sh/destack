from django.db import models

from bench.models.utils import MAX_DESCRIPTION_LENGTH, MAX_NAME_LENGTH, UUIDModel


class Tag(UUIDModel):
    """
    A generic label for associating groups and/or parts of primitives like
    artifacts, functions and executions with some metadata.
    """

    type = models.CharField(max_length=64)
    name = models.CharField(max_length=MAX_NAME_LENGTH)
    description = models.CharField(max_length=MAX_DESCRIPTION_LENGTH, blank=True)
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    metadata = models.JSONField(default={})


class Alias(Tag):
    class Meta:
        proxy = True


class Capability(Tag):
    class Meta:
        proxy = True


class Stage(Tag):
    class Meta:
        proxy = True
