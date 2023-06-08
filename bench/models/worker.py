from __future__ import annotations

from django.db import models

from bench.models.utils import UUIDModel


class WorkerTenancy(models.TextChoices):
    COMMUNITY = "COMMUNITY"
    OWNER = "DEDICATED"


class WorkerStatus(models.TextChoices):
    STARTING = "STARTING"
    ACTIVE = "ACTIVE"
    STOPPING = "STOPPING"
    TERMINATED = "TERMINATED"


class Worker(UUIDModel):
    """A worker is a worker node in a deployment."""

    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    started_at = models.DateTimeField(null=True)
    terminated_at = models.DateTimeField(null=True)
    tenancy = models.CharField(max_length=20, choices=WorkerTenancy.choices)
    status = models.CharField(max_length=20, choices=WorkerStatus.choices)
    project = models.ForeignKey(
        "Project", on_delete=models.CASCADE, related_name="workers", null=True
    )
    last_seen_at = models.DateTimeField(null=True)

    def __str__(self):
        return f"{self.id} {self.status} ({self.tenancy}1)"

    def __repr__(self):
        return f"<SandboxedWorker {self}>"
