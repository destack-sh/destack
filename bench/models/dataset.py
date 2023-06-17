from __future__ import annotations

from typing import Optional
from uuid import UUID, uuid5

from django.db import models

from bench import bench as lang
from bench.bench.const import DatasetBackend
from bench.models.utils import CrudModel, ModuleNode, UUIDModel, get_choices


class Dataset(UUIDModel, ModuleNode):
    """A user created dataset."""

    statement: Statement  # noqa via Statement.dataset
    backend = models.CharField(max_length=64, choices=get_choices(DatasetBackend))
    backend_id = models.CharField(max_length=64)
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    versioned = models.BooleanField(default=True)

    @property
    def parent_id(self) -> Optional[UUID]:
        return self.statement.id

    @staticmethod
    def get_id(statement: "Statement") -> UUID:
        return uuid5(statement.id, "dataset")


class DatasetView(UUIDModel, CrudModel):
    """A view of a dataset."""

    dataset = models.ForeignKey("Dataset", on_delete=models.CASCADE, related_name="views")
    layout = models.CharField(max_length=64, choices=get_choices(lang.DatasetViewLayout))
    query = models.JSONField()
    sort = models.JSONField()
    fields: models.QuerySet[DatasetViewField]  # noqa via DatasetViewField.view


class DatasetViewField(UUIDModel):
    view = models.ForeignKey("DatasetView", on_delete=models.CASCADE, related_name="fields")
    field = models.ForeignKey("Field", on_delete=models.CASCADE, related_name="views+")
    order_key = models.CharField(max_length=64, null=True, blank=True)
    visible = models.BooleanField(default=True)
