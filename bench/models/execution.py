from __future__ import annotations

from typing import Optional

from django.db import models
from django_choices_field import TextChoicesField
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


# sync with wire.ExecutionTriggerType
class ExecutionTriggerType(models.TextChoices):
    REST_API = "rest-api"
    UI_INTERACTIVE = "ui-interactive"
    JOB = "job"
    MANUAL = "manual"  # catch-all for old/debug triggers


# sync with wire.ExecutionTracingLevel
class ExecutionTracingLevel(models.TextChoices):
    ROOT_FRAME = "root-frame"
    ROOT_FRAME_WITH_DATA = "root-frame-with-data"
    ALL_FRAMES = "all-frames"
    ALL_FRAMES_WITH_DATA = "all-frames-with-data"


class Execution(UUIDTModel):
    """
    The execution of (nested) code.
    """

    # context
    project_version = models.ForeignKey(
        "ProjectVersion", on_delete=models.CASCADE, related_name="executions+"
    )
    deployment = models.ForeignKey(
        "Deployment", on_delete=models.CASCADE, related_name="executions+"
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

    tracing_level = TextChoicesField(choices_enum=ExecutionTracingLevel)
    trigger_type = TextChoicesField(choices_enum=ExecutionTriggerType)
    user = models.ForeignKey(
        "User", null=True, blank=True, on_delete=models.SET_NULL, related_name="executions+"
    )
    access_token = models.ForeignKey(
        "AccessToken", null=True, blank=True, on_delete=models.SET_NULL, related_name="executions+"
    )

    # execution
    root = models.ForeignKey(
        "Execution", on_delete=models.CASCADE, null=True, blank=True, related_name="descendants"
    )
    parent = models.ForeignKey(
        "Execution", on_delete=models.CASCADE, null=True, blank=True, related_name="children"
    )
    build = models.ForeignKey(
        "Statement",
        null=True,
        blank=True,
        on_delete=models.SET_NULL,
        related_name="executions+",
    )
    task = models.ForeignKey(
        "Statement",
        null=True,
        blank=True,
        on_delete=models.SET_NULL,
        related_name="executions+",
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
        root_str = f"root={self.root_id}" if self.root_id else ""
        parent_str = f"parent={self.parent_id}" if self.parent_id else ""
        return f"{self.id} {self.status} ({(root_str + ' ' + parent_str).strip()})"

    class Meta:
        ordering = ["-created_at"]
