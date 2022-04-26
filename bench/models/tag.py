from django.db import models

from bench.models.utils import MAX_DESCRIPTION_LENGTH, MAX_NAME_LENGTH, UUIDModel


class Tag(UUIDModel):
    """
    A generic tag for labeling and associating groups and/or parts of primitives like
    artifacts and executions.
    """

    type = models.CharField(max_length=256)
    name = models.CharField(max_length=MAX_NAME_LENGTH)
    description = models.CharField(max_length=MAX_DESCRIPTION_LENGTH, null=True)
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)


class Alias(Tag):
    class Meta:
        proxy = True


class Capability(Tag):
    class Meta:
        proxy = True


class Stage(Tag):
    class Meta:
        proxy = True
