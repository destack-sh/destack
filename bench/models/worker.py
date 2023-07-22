from __future__ import annotations

from typing import TYPE_CHECKING

from django.db import models

from bench.language.session import WorkerProfile, WorkerRegion, WorkerSetStatus
from bench.models.utils import UUIDModel, get_choices

if TYPE_CHECKING:
    from bench.models.project import Project


class WorkerSet(UUIDModel):
    """A desired-state set of homogenous workers for a project. Maps to/from k8 deployments."""

    project: "Project"  # noqa via Project.worker_set
    project_id = models.ForeignKey("Project", on_delete=models.CASCADE, related_name="worker_sets")
    region = models.CharField(max_length=32, choices=get_choices(WorkerRegion))
    profile = models.CharField(max_length=32, choices=get_choices(WorkerProfile))
    sleeping = models.BooleanField(default=False)
    status = models.CharField(max_length=32, choices=get_choices(WorkerSetStatus))
    desired_replicas = models.IntegerField(default=0)
    target_replicas = models.IntegerField(default=0)
    available_replicas = models.IntegerField(default=0)
    ready_replicas = models.IntegerField(default=0)
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    last_active_at = models.DateTimeField(null=True, blank=True)

    def __str__(self):
        return f"{self.project} ({self.region}, {self.profile}, {self.status}, x{self.desired_replicas})"

    def __repr__(self):
        return f"<WorkerSet {self}>"


WORKER_SET_FIELDS: list[str] = [f.name for f in WorkerSet._meta.fields]
