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


TERMINAL_EXECUTION_STATUSES = {
    ExecutionStatus.Aborted,
    ExecutionStatus.Failed,
    ExecutionStatus.Completed,
}
PENDING_EXECUTION_STATUSES = set(ExecutionStatus) - TERMINAL_EXECUTION_STATUSES


class ExecutionTriggerType(models.TextChoices):
    API = "rest"
    UI = "ui"
    REACTIVE = "reactive"
    SCHEDULED = "scheduled"


class Execution(UUIDTModel):
    """
    The execution of (nested) code.
    """

    # context
    project = models.ForeignKey("Project", on_delete=models.CASCADE)
    project_version = models.ForeignKey("ProjectVersion", on_delete=models.CASCADE)
    worker = models.ForeignKey("Worker", null=True, blank=True, on_delete=models.SET_NULL)
    tracing_level = models.IntegerField(default=0)
    trigger_type = models.CharField(
        max_length=32, choices=ExecutionTriggerType.choices, default=ExecutionTriggerType.API
    )
    user = models.ForeignKey("User", null=True, blank=True, on_delete=models.SET_NULL)
    access_token = models.ForeignKey(
        "AccessToken", null=True, blank=True, on_delete=models.SET_NULL
    )

    # content
    status = models.CharField(
        max_length=32, choices=ExecutionStatus.choices, default=ExecutionStatus.Created
    )
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    started_at = models.DateTimeField(null=True, blank=True)
    terminated_at = models.DateTimeField(null=True, blank=True)
    cached_generated_at = models.DateTimeField(null=True, blank=True)
    cached_duration = models.FloatField(null=True, blank=True)
    root = models.ForeignKey(
        "Execution", on_delete=models.CASCADE, null=True, blank=True, related_name="descendants"
    )
    parent = models.ForeignKey(
        "Execution", on_delete=models.CASCADE, null=True, blank=True, related_name="children"
    )
    runnable = models.ForeignKey("Statement", null=True, blank=True, on_delete=models.SET_NULL)
    inputs = models.JSONField(null=True, blank=True)
    outputs = models.JSONField(null=True, blank=True)
    error = models.JSONField(null=True, blank=True)
    metadata = models.JSONField(null=True, blank=True)

    @gql.model_property(only=["started_at", "terminated_at"])
    def duration(self) -> Optional[float]:
        if self.started_at and self.terminated_at:
            return (self.terminated_at - self.started_at).total_seconds()
        return None

    def __str__(self):
        root_str = f"root={self.root_id}" if self.root_id else ""
        parent_str = f"parent={self.parent_id}" if self.parent_id else ""
        return f"{self.id} {self.status} ({(root_str + ' ' + parent_str).strip()})"

    class Meta:
        ordering = ["-created_at"]
