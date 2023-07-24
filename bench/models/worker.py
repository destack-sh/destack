from __future__ import annotations

from typing import TYPE_CHECKING

from django.db import models

from bench.language.session import WorkerProfile, WorkerRegion, WorkerSetStatus
from bench.models.utils import UUIDModel, get_choices

if TYPE_CHECKING:
    pass


class WorkerSet(UUIDModel):
    """A desired-state set of homogenous workers for a project. Maps to/from k8 deployments."""

    project = models.ForeignKey("Project", on_delete=models.CASCADE, related_name="worker_sets")
    region = models.CharField(max_length=32, choices=get_choices(WorkerRegion))
    profile = models.CharField(max_length=32, choices=get_choices(WorkerProfile))
    sleeping = models.BooleanField(default=False)
    desired_replicas = models.IntegerField(default=0)
    target_replicas = models.IntegerField(default=0)
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    last_active_at = models.DateTimeField(null=True, blank=True)
    # ready from k8 only
    status = models.CharField(max_length=32, choices=get_choices(WorkerSetStatus))
    available_replicas = models.IntegerField(default=0)
    ready_replicas = models.IntegerField(default=0)

    def __str__(self):
        return f"{self.project} ({self.region}, {self.profile}, {self.status}, x{self.desired_replicas})"

    def __repr__(self):
        return f"<WorkerSet {self}>"


WORKER_SET_FIELDS: list[str] = [f.name for f in WorkerSet._meta.fields]
WORKER_SET_FIELDS_NO_ID: list[str] = [f for f in WORKER_SET_FIELDS if f != "id"]
