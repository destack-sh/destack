from __future__ import annotations

from typing import Any

from django.db import models

from bench.models.utils import UUIDModel


class Compilation(UUIDModel):
    """
    A compilation translates a task with a template code tree (code) into a runnable code.

    Depending on the compilation target and options, various optimizations may be applied.
    """

    mappings: models.QuerySet["SourceMapping"]  # noqa via SourceMapping.compilation

    def deepcopy(self, to, refs: dict[str, Any]):
        pass  # nothing to do

    def __str__(self):
        return f"{self.id.hex}.compilation"


class SourceMapping(UUIDModel):
    """
    A source map for compilations to track the mapping between source and target instructions.
    """

    compilation = models.ForeignKey(
        "Compilation", on_delete=models.CASCADE, related_name="mappings"
    )
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
