from __future__ import annotations

from typing import Optional
from uuid import UUID

from django.db import models
from strawberry_django_plus import gql

from bench.models.schema import SchemaElementField, SchemaField
from bench.models.symbol import SymbolContent, SymbolContentManager, SymbolDefinition
from bench.models.utils import UUIDTModel


class CodeManager(SymbolContentManager, models.Manager["Code"]):
    pass


class Code(SymbolContent):
    """
    Code specifies how to do something using datasets, models and other code.
    Code is just async Python code (either defined in-place or as a built-in).
    """

    input_schema = SchemaField("input")
    output_schema = SchemaElementField("output")
    # either set builtin id or set custom code
    builtin_id = models.CharField(null=True, blank=True, max_length=256)
    code = models.TextField(null=True, blank=True)
    code_function_name = models.CharField(null=True, blank=True, max_length=256)
    tasks = models.ManyToManyField("Task", related_name="implementations")

    def deepcopy(self, to: Code, refs: dict[UUID, SymbolDefinition | SymbolContent]):
        super().deepcopy(to, refs)

    def __str__(self):
        if self.builtin_id:
            content = f"builtin={self.builtin_id}"
        elif self.code_function_name and self.code:
            content = f"function={self.code_function_name},chars={len(self.code)},lines={len(self.code.splitlines())}"
        elif self.code:
            content = f"length={len(self.code)}"
        else:
            raise ValueError(f"code has no content: {self}")
        return f"{self.definition_str}({content},{self.input_schema}->{self.output_schema})"

    @property
    def anonymous(self) -> bool:
        """Whether this code is defined as anonymous code ir with a defined function (same name)"""
        if self.builtin_id is not None:
            # builtins are always directly callable
            return False
        elif self.code is not None:
            return self.code_function_name is None
        else:
            raise ValueError(f"code {self} must have either builtin_id or code")

    objects = CodeManager()

    class Meta(SymbolContent.Meta):
        constraints = [
            # ensure either builtin_id or code is set
            models.CheckConstraint(
                name="bench_code_builtin_id_xor_code_ck",
                check=(models.Q(builtin_id__isnull=False) ^ models.Q(code__isnull=False)),
            ),
        ]


class ExecutionStatus(models.TextChoices):
    Created = "created"
    Scheduled = "scheduled"
    Queued = "queued"
    Running = "running"
    Aborting = "aborting"
    # terminal statuses
    Aborted = "aborted"
    Failed = "failed"
    Completed = "completed"


TERMINAL_STATUSES = {ExecutionStatus.Aborted, ExecutionStatus.Failed, ExecutionStatus.Completed}
PENDING_STATUSES = set(ExecutionStatus) - TERMINAL_STATUSES


class Execution(UUIDTModel):
    """
    The execution of (hierarchical) code.
    """

    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    started_at = models.DateTimeField(
        null=True, blank=True, help_text="Time of transition to RUNNING status."
    )
    terminated_at = models.DateTimeField(
        null=True, blank=True, help_text="Time of transition to a terminal status."
    )
    status = models.CharField(
        max_length=32, choices=ExecutionStatus.choices, default=ExecutionStatus.Created
    )

    parent = models.ForeignKey(
        "Execution", on_delete=models.CASCADE, null=True, blank=True, related_name="children"
    )
    code = models.ForeignKey(
        "code",
        null=True,
        blank=True,
        on_delete=models.SET_NULL,
        related_name="executions",
    )
    model = models.ForeignKey(
        "Model", null=True, blank=True, on_delete=models.SET_NULL, related_name="executions"
    )
    model_inference = models.ForeignKey(
        "ModelInference",
        null=True,
        blank=True,
        on_delete=models.SET_NULL,
        related_name="executions",
    )
    inputs = models.JSONField(null=True, blank=True)
    outputs = models.JSONField(null=True, blank=True)
    error = models.JSONField(null=True, blank=True)

    @gql.model_property(only=["started_at", "terminated_at"])
    def duration_millis(self) -> Optional[float]:
        if self.started_at and self.terminated_at:
            return (self.terminated_at - self.started_at).total_seconds() * 1000
        return None
