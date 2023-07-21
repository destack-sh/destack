from __future__ import annotations

from django.db import models

from bench.language.session import WorkerProfile, WorkerRegion, WorkerSetStatus, WorkerStatus
from bench.models.utils import UUIDModel, get_choices


class WorkerSet(UUIDModel):
    """A desired set of homogenous workers for a project. Maps to k8 deployments."""

    project = models.OneToOneField("Project", on_delete=models.CASCADE, related_name="worker_set")
    region = models.CharField(max_length=32, choices=get_choices(WorkerRegion))
    profile = models.CharField(max_length=32, choices=get_choices(WorkerProfile))
    status = models.CharField(max_length=32, choices=get_choices(WorkerSetStatus))
    target_count = models.IntegerField(default=0)
    actual_count = models.IntegerField(default=0)
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    last_active_at = models.DateTimeField(null=True, blank=True)
    k8_deployment_id = models.CharField(max_length=64, null=True, blank=True)


class WorkerNode(UUIDModel):
    """The worker nodes that make up a worker set. Masp to k8 pods."""

    worker_set = models.ForeignKey("WorkerSet", on_delete=models.CASCADE, related_name="nodes")
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    started_at = models.DateTimeField(null=True, blank=True)
    terminated_at = models.DateTimeField(null=True, blank=True)
    last_seen_at = models.DateTimeField(null=True, blank=True)
    last_active_at = models.DateTimeField(null=True, blank=True)
    profile = models.CharField(max_length=32, choices=get_choices(WorkerProfile))
    status = models.CharField(max_length=32, choices=get_choices(WorkerStatus))
    k8_pod_id = models.CharField(max_length=64, null=True, blank=True)
