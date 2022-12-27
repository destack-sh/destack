from __future__ import annotations

from typing import Any

from django.db import models

from bench.models.symbol import Statement, SymbolType
from bench.models.utils import UUIDModel


class Compilation(UUIDModel):
    """
    A compilation translates a task with a template code tree (code) into a runnable code.

    Depending on the compilation target and options, various optimizations may be applied.
    """

    mappings: models.QuerySet["SourceMapping"]  # noqa via SourceMapping.compilation
    definition: Statement  # noqa via Statement.compilation

    def deepcopy(self, to, refs: dict[str, Any]):
        pass  # nothing to do

    @property
    def task(self) -> Statement:
        if self.definition.reference is None:
            raise ValueError("compilation has no reference")
        if self.definition.reference.symbol_type != SymbolType.TASK:
            raise ValueError("compilation has no task")
        return self.definition.reference

    @property
    def model_backends(self) -> models.QuerySet[Statement]:
        return self.definition.arguments.filter(symbol_type=SymbolType.MODEL)

    def __str__(self):
        return f"(compile)"


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
