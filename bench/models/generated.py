from __future__ import annotations

from django.db import models

from bench.models.utils import UUIDModel


class GeneratedContentMixin:
    """Build content of mappings."""

    generated_mappings: models.QuerySet["SourceMapping"]  # noqa via SourceMapping.build


class SourceMapping(UUIDModel):
    """
    A source map for builds to track dependencies of generated content.
    """

    statement = models.ForeignKey(
        "Statement", on_delete=models.CASCADE, related_name="generated_mappings"
    )
    source = models.ForeignKey(
        "Statement", on_delete=models.CASCADE, related_name="target_mappings"
    )
    source_revision = models.IntegerField()
    target = models.ForeignKey(
        "Statement", on_delete=models.CASCADE, related_name="source_mappings", null=True, blank=True
    )
    target_revision = models.IntegerField(null=True, blank=True)
