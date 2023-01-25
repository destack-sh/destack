from __future__ import annotations

from django.db import models

from bench.models.utils import UUIDModel


class CompilationContentMixin:
    """Compilation content of mappings."""

    generated_mappings: models.QuerySet["SourceMapping"]  # noqa via SourceMapping.compilation


class SourceMapping(UUIDModel):
    """
    A source map for compilations to track the mapping between source and target instructions.
    """

    compilation = models.ForeignKey("Statement", on_delete=models.CASCADE, related_name="mappings")
    source = models.ForeignKey(
        "Statement", on_delete=models.CASCADE, related_name="target_mappings"
    )
    source_revision = models.IntegerField()
    source_path = models.JSONField()
    target = models.ForeignKey(
        "Statement", on_delete=models.CASCADE, related_name="source_mappings"
    )
    target_path = models.JSONField()
    target_revision = models.IntegerField()
