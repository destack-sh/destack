from __future__ import annotations

import uuid
from typing import TYPE_CHECKING, Optional

from django.db import models

from bench.language.const import DatasetViewLayout
from bench.models.utils import (
    CrudModel,
    DetachedModuleNode,
    ModuleNode,
    Revisioned,
    UUIDModel,
    get_choices,
)

if TYPE_CHECKING:
    from bench.models.statement import Statement


class DatasetView(CrudModel, ModuleNode):
    """A view of a dataset."""

    statement = models.ForeignKey("Statement", on_delete=models.CASCADE, related_name="views")
    layout = models.CharField(max_length=64, choices=get_choices(DatasetViewLayout))
    query = models.JSONField()
    sort = models.JSONField()
    fields: models.QuerySet[DatasetViewField]  # noqa via DatasetViewField.view

    @property
    def parent_id(self) -> Optional[uuid.UUID]:
        return self.statement_id

    @property
    def parent(self) -> Optional["Statement"]:
        return self.statement


class DatasetViewField(CrudModel, ModuleNode):
    view = models.ForeignKey("DatasetView", on_delete=models.CASCADE, related_name="fields")
    field = models.ForeignKey("Field", on_delete=models.CASCADE, related_name="views+")
    order_key = models.CharField(max_length=64, null=True, blank=True)
    visible = models.BooleanField(default=True)

    @property
    def parent_id(self) -> Optional[uuid.UUID]:
        return self.view_id

    @property
    def parent(self) -> Optional["DatasetView"]:
        return self.view


class Record(CrudModel, DetachedModuleNode, Revisioned):
    """
    A record in a dataset (may be detached if the dataset is not versioned). Currently unused :DbRecord
    We may choose not to store the actual record value here later, but for now it's convenient.
    """

    statement = models.ForeignKey(
        "Statement", on_delete=models.CASCADE, null=True, related_name="records+"
    )
    statement_ck = models.UUIDField()
    value = models.JSONField(null=True, blank=True)

    @property
    def parent_id(self) -> Optional[uuid.UUID]:
        return self.statement_id

    @property
    def parent(self) -> Optional["Statement"]:
        return self.statement


class RecordRelation(UUIDModel):
    """
    Relation between a Record and something (another Record, a Statement, a Run, etc.).
    (Not actually used yet, this is just me thinking out loud.)
    """

    # relation 'parent'
    project_version = models.ForeignKey(
        "ProjectVersion", on_delete=models.CASCADE, related_name="+"
    )
    parent = models.ForeignKey("Record", on_delete=models.CASCADE, related_name="+")
    type = models.CharField(max_length=64)
    path = models.CharField(max_length=128, null=True, blank=True)
    # relation 'child'
    record_ck = models.UUIDField(null=True)
    statement_ck = models.UUIDField(null=True)
    run = models.ForeignKey("Run", on_delete=models.CASCADE, null=True, related_name="+")
    remote_object = models.ForeignKey("RemoteObject", on_delete=models.CASCADE, null=True)
    secret = models.ForeignKey("Secret", on_delete=models.CASCADE, null=True)
