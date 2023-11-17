from __future__ import annotations

import uuid
from typing import TYPE_CHECKING, Optional

from django.db import models

from bench.models.utils import CrudModel, DetachedModuleNode, Revisioned

if TYPE_CHECKING:
    from bench.models.statement import Statement


class RecordManager(models.Manager):
    def get_queryset(self):
        return super().get_queryset().filter(deleted_at__isnull=True)


class Record(CrudModel, DetachedModuleNode, Revisioned):
    """
    nocheckin: 7. remove Record model
    A record in a database (may be detached if the database is not versioned).
    We may choose not to store the actual record value here later, but for now it's convenient.
    """

    statement = models.ForeignKey(
        "Statement", on_delete=models.CASCADE, null=True, related_name="records+"
    )
    statement_ck = models.UUIDField()
    statement_key = models.CharField(max_length=64)
    value = models.JSONField(null=True, blank=True)

    @property
    def parent_id(self) -> Optional[uuid.UUID]:
        return self.statement_id

    @property
    def parent(self) -> Statement:
        return self.statement

    objects = RecordManager()

    class Meta:
        indexes = []
