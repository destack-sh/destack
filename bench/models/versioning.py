from django.db import models

from bench.models.utils import MAX_DESCRIPTION_LENGTH, MAX_NAME_LENGTH


class VersionedRepository(models.Model):
    """
    A versioned repository similar to Git.
    """

    class Meta:
        abstract = True


class VersionedObject(models.Model):
    class Meta:
        abstract = True


class VersionedBlob(VersionedObject):
    class Meta:
        abstract = True


class VersionedTree(VersionedObject):
    class Meta:
        abstract = True


class VersionedCommit(VersionedObject):
    version = models.CharField(max_length=256)
    name = models.CharField(max_length=MAX_NAME_LENGTH, null=True, blank=True)
    description = models.CharField(
        max_length=MAX_DESCRIPTION_LENGTH, null=True, blank=True
    )
    created_at = models.DateTimeField(auto_now_add=True)

    class Meta:
        abstract = True
