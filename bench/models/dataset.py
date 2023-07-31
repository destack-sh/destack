from __future__ import annotations

import uuid
from typing import TYPE_CHECKING, Optional
from uuid import UUID, uuid5

from django.db import models

from bench.language.const import DatasetViewLayout
from bench.models.utils import CrudModel, ModuleNode, Revisioned, UUIDModel, get_choices

if TYPE_CHECKING:
    from bench.models.statement import Statement


class Dataset(UUIDModel, ModuleNode):
    """A user created dataset."""

    statement: Statement  # noqa via Statement.dataset
    key = models.CharField(max_length=64)
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    versioned = models.BooleanField(default=True)

    @property
    def parent_id(self) -> Optional[UUID]:
        return self.statement.id

    @staticmethod
    def get_id(statement: "Statement") -> UUID:
        return uuid5(statement.id, "dataset")


class DatasetView(UUIDModel, CrudModel, ModuleNode):
    """A view of a dataset."""

    dataset = models.ForeignKey("Dataset", on_delete=models.CASCADE, related_name="views")
    layout = models.CharField(max_length=64, choices=get_choices(DatasetViewLayout))
    query = models.JSONField()
    sort = models.JSONField()
    fields: models.QuerySet[DatasetViewField]  # noqa via DatasetViewField.view

    @property
    def parent_id(self) -> Optional[uuid.UUID]:
        return self.dataset_id

    @property
    def parent(self) -> Optional["Dataset"]:
        return self.dataset


class DatasetViewField(UUIDModel, CrudModel, ModuleNode):
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


class Record(UUIDModel, CrudModel, ModuleNode, Revisioned):
    """
    A record in a dataset (may be untethered if the dataset is not versioned). Currently unused :DbRecord
    We may choose not to store the actual record value here later, but for now it's convenient.
    """

    dataset = models.ForeignKey(
        "Dataset", on_delete=models.CASCADE, related_name="records", null=True
    )
    dataset_key = models.CharField(max_length=64)
    value = models.JSONField(null=True, blank=True)

    class Meta:
        indexes = [models.Index(fields=["dataset_key"], name="bench_record_dataset_key_idx")]


class RecordRelation(UUIDModel):
    """
    Relation between a Record and something (another Record, a Statement, a Run, etc.).
    (Not actually used yet, this is just me thinking out loud.)
    """

    parent = models.ForeignKey("Record", on_delete=models.CASCADE, related_name="+")
    type = models.CharField(max_length=64)
    record = models.ForeignKey("Record", on_delete=models.CASCADE, null=True, related_name="+")
    statement = models.ForeignKey(
        "Statement", on_delete=models.CASCADE, null=True, related_name="+"
    )
    run = models.ForeignKey("Run", on_delete=models.CASCADE, null=True, related_name="+")
    remote_object = models.ForeignKey("RemoteObject", on_delete=models.CASCADE, null=True)
    secret = models.ForeignKey("Secret", on_delete=models.CASCADE, null=True)
    path = models.CharField(max_length=128, null=True, blank=True)
