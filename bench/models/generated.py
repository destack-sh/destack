from __future__ import annotations

from django.db import models

from bench.models.utils import UUIDModel


class GeneratedContentMixin:
    """Build content of mappings."""

    generated_mappings: models.QuerySet["SourceMapping"]  # noqa via SourceMapping.build


class SourceMapping(UUIDModel):
    """
    A source map for builds to track the mapping between source and target instructions.
    """

    statement = models.ForeignKey(
        "Statement", on_delete=models.CASCADE, related_name="generated_mappings"
    )
    source = models.ForeignKey(
        "Statement", on_delete=models.CASCADE, related_name="target_mappings"
    )
    source_revision = models.IntegerField()
    source_path = models.JSONField(null=True, blank=True)
    target = models.ForeignKey(
        "Statement", on_delete=models.CASCADE, related_name="source_mappings"
    )
    target_path = models.JSONField(null=True, blank=True)
    target_revision = models.IntegerField()
