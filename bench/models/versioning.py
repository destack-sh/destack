"""
We use Git-inspired versioning for our models.

We follow Git in using repositories, references and objects:
 - Repositories are top-level containers of data
  - Repositories may refer to other repositories as with "submodules"
 - Objects are "blobs", "trees" or "commits"
  - Blobs are entire files, the atomic units of versioning
  - Trees are indexed listings of blobs or other subtrees
  - Commits are named references to a root tree

We deviate from Git in that:
 - We are effectively centralised, not distributed.
 - Objects are indexed by their hash, but primary/foreign keys use UUIDs.
 - Objects may contain metadata invisible to the versioning process.
 - Objects may be marked 'mutable' before becoming immutable forever.
"""

from django.db import models

from bench.models.utils import MAX_DESCRIPTION_LENGTH, MAX_NAME_LENGTH


class VersionedRepository(models.Model):
    """
    A versioned repository containing versioned objects, similar to Git.
    """

    class Meta:
        abstract = True


class VersionedObject(models.Model):
    """
    Any object involved in versioning of a repository.

    Unlike in Git, objects may contain fields not related to their versioned state.
    """

    # TODO @Robustness: hash object content in Postgres directly?
    #  See https://www.postgresql.org/docs/11/functions-binarystring.html
    content_hash = models.BinaryField(max_length=32)
    mutable = models.BooleanField(default=False)

    class Meta:
        abstract = True


class VersionedBlob(VersionedObject):
    """
    An atomic and generally immutable piece of data.
    """

    class Meta:
        abstract = True


class VersionedTree(VersionedObject):
    """
    A listing of paths to blobs and subtrees.
    """

    class Meta:
        abstract = True


class VersionedCommit(VersionedObject):
    """
    A point-in-time snapshot of a repository.
    """

    name = models.CharField(max_length=MAX_NAME_LENGTH, null=True, blank=True)
    description = models.CharField(
        max_length=MAX_DESCRIPTION_LENGTH, null=True, blank=True
    )
    created_at = models.DateTimeField(auto_now_add=True)

    class Meta:
        abstract = True
