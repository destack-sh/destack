from __future__ import annotations

from typing import Optional

from django.db import models
from strawberry_django_plus import gql

from bench.language.const import RunTriggerType
from bench.language.session import RunStatus
from bench.models.utils import UUIDTModel, get_choices


class Session(UUIDTModel):
    project_version = models.ForeignKey("ProjectVersion", on_delete=models.CASCADE)
    worker_node = models.ForeignKey("WorkerNode", null=True, blank=True, on_delete=models.SET_NULL)
    trigger_type = models.CharField(max_length=32, choices=get_choices(RunTriggerType))
    user = models.ForeignKey("User", null=True, blank=True, on_delete=models.SET_NULL)
    access_token = models.ForeignKey(
        "AccessToken", null=True, blank=True, on_delete=models.SET_NULL
    )
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    opened_at = models.DateTimeField(null=True, blank=True)
    closed_at = models.DateTimeField(null=True, blank=True)
    metadata = models.JSONField(null=True, blank=True)


class Run(UUIDTModel):
    project_version = models.ForeignKey("ProjectVersion", on_delete=models.CASCADE)
    session = models.ForeignKey("Session", on_delete=models.CASCADE, related_name="runs")
    status = models.CharField(max_length=32, choices=get_choices(RunStatus))
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    started_at = models.DateTimeField(null=True, blank=True)
    terminated_at = models.DateTimeField(null=True, blank=True)
    root = models.ForeignKey(
        "Run", on_delete=models.CASCADE, null=True, blank=True, related_name="descendants"
    )
    parent = models.ForeignKey(
        "Run", on_delete=models.CASCADE, null=True, blank=True, related_name="children"
    )
    runnable = models.ForeignKey("Statement", null=True, blank=True, on_delete=models.SET_NULL)
    runnable_type = models.CharField(max_length=64, null=True, blank=True)
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
        indexes = [
            models.Index(fields=["started_at"], name="run_started_at_idx"),
            models.Index(fields=["updated_at"], name="run_updated_at_idx"),
            models.Index(fields=["runnable_id"], name="run_runnable_id_idx"),
            models.Index(
                fields=["runnable_id", "project_version_id"], name="run_runnable_id_scoped_idx"
            ),
        ]


# LogEntry lives in OpenSearch only
