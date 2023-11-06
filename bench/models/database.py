from __future__ import annotations

import uuid
from typing import TYPE_CHECKING, Optional

from django.db import models

from bench.language.const import ViewLayout
from bench.models.utils import CrudModel, CrudNode, DetachedModuleNode, Revisioned, get_choices
from bench.utils.dt import utcnow_with_tz

if TYPE_CHECKING:
    from bench.models.statement import Statement


class DatabaseViewManager(models.Manager):
    def get_queryset(self):
        return super().get_queryset().filter(deleted_at__isnull=True)


class DatabaseView(CrudNode):
    """A view of a database."""

    statement = models.ForeignKey("Statement", on_delete=models.CASCADE, related_name="views")
    layout = models.CharField(max_length=64, choices=get_choices(ViewLayout))
    query = models.JSONField()
    sort = models.JSONField()
    fields: models.QuerySet[DatabaseViewField]  # noqa via DatabaseViewField.view

    @property
    def parent_id(self) -> Optional[uuid.UUID]:
        return self.statement_id

    @property
    def parent(self) -> Optional["Statement"]:
        return self.statement

    objects = DatabaseViewManager()


class DatabaseViewFieldManager(models.Manager):
    def get_queryset(self):
        return super().get_queryset().filter(deleted_at__isnull=True)


class DatabaseViewField(CrudNode):
    view = models.ForeignKey("DatabaseView", on_delete=models.CASCADE, related_name="fields")
    field = models.ForeignKey("Field", on_delete=models.CASCADE, related_name="views+")
    order_key = models.CharField(max_length=64, null=True, blank=True)
    visible = models.BooleanField(default=True)

    @property
    def parent_id(self) -> Optional[uuid.UUID]:
        return self.view_id

    @property
    def parent(self) -> Optional["DatabaseView"]:
        return self.view

    objects = DatabaseViewFieldManager()


class RecordManager(models.Manager):
    def get_queryset(self):
        return super().get_queryset().filter(deleted_at__isnull=True)


class Record(CrudModel, DetachedModuleNode, Revisioned):
    """
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

    def soft_delete(self):
        self.deleted_at = utcnow_with_tz()

    def restore(self):
        self.deleted_at = None

    objects = RecordManager()

    class Meta:
        indexes = []
