from __future__ import annotations

from typing import Optional
from uuid import UUID

from django.db import models

from bench.models.utils import ModuleNode, UUIDModel


class DatasetBackend(models.TextChoices):
    """The backend used to store the dataset."""

    OPENSEARCH = "OPENSEARCH"


class Dataset(UUIDModel, ModuleNode):
    """A user created dataset backing the data symbol of a statement."""

    statement: Statement  # noqa via Statement.dataset
    backend = models.CharField(max_length=64, choices=DatasetBackend.choices)
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    versioned = models.BooleanField(default=True)
    # opensearch
    os_mappings: models.QuerySet[OpensearchMapping]  # noqa via OpensearchMapping.dataset

    @property
    def parent_id(self) -> Optional[UUID]:
        return self.statement.id


class OpensearchMapping(models.Model):
    """An OpenSearch field mapping for a dataset."""

    id = models.IntegerField(primary_key=True)
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    deleted_at = models.DateTimeField(null=True)
    dataset = models.ForeignKey("Dataset", on_delete=models.CASCADE, related_name="os_mappings")
    mapping = models.JSONField()
    field = models.ForeignKey(
        "Field", on_delete=models.SET_NULL, related_name="os_mappings", null=True
    )
