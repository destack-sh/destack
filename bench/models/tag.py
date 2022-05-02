from django.db import models

from bench.models.utils import MAX_DESCRIPTION_LENGTH, MAX_NAME_LENGTH, UUIDModel


class Tag(UUIDModel):
    """
    A generic label for associating groups and/or parts of primitives like
    artifacts, functions and executions with some metadata.
    """

    # TODO @Feature: version (some) tags?
    #  Where would versioned tags be needed? Is copy-on-write enough?

    type = models.CharField(max_length=64)
    name = models.CharField(max_length=MAX_NAME_LENGTH)
    description = models.CharField(
        max_length=MAX_DESCRIPTION_LENGTH, null=True, blank=True
    )
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    metadata = models.JSONField(default=dict)


class TaggedItem(UUIDModel):
    pass


class Alias(Tag):
    class Meta:
        proxy = True


class Capability(Tag):
    class Meta:
        proxy = True


class Stage(Tag):
    class Meta:
        proxy = True
