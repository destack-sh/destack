from __future__ import annotations

from typing import Optional

from django.db import models
from strawberry_django_plus import gql

from bench.models.utils import UUIDTModel


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

    project_version = models.ForeignKey(
        "ProjectVersion", on_delete=models.CASCADE, related_name="executions+"
    )
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

    root = models.ForeignKey(
        "Execution", on_delete=models.CASCADE, null=True, blank=True, related_name="descendants"
    )
    parent = models.ForeignKey(
        "Execution", on_delete=models.CASCADE, null=True, blank=True, related_name="children"
    )
    code = models.ForeignKey(
        "Statement",
        null=True,
        blank=True,
        on_delete=models.SET_NULL,
        related_name="executions+",
    )
    model = models.ForeignKey(
        "Statement", null=True, blank=True, on_delete=models.SET_NULL, related_name="executions+"
    )
    model_inference = models.ForeignKey(
        "ModelInference",
        null=True,
        blank=True,
        on_delete=models.SET_NULL,
        related_name="executions+",
    )
    inputs = models.JSONField(null=True, blank=True)
    outputs = models.JSONField(null=True, blank=True)
    error = models.JSONField(null=True, blank=True)

    @gql.model_property(only=["started_at", "terminated_at"])
    def duration_millis(self) -> Optional[float]:
        if self.started_at and self.terminated_at:
            return (self.terminated_at - self.started_at).total_seconds() * 1000
        return None

    def __str__(self):
        return f"{self.id} {self.status}"
