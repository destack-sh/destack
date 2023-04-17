from django.db import models
from django_choices_field import TextChoicesField

from bench.models.utils import UUIDModel


class JobType(models.TextChoices):
    INTERP = "interp"
    GENERATE = "generate"
    BUILD = "build"
    EVALUATE = "evaluate"
    LINT = "lint"


class JobStatus(models.TextChoices):
    Queued = "queued"
    Running = "running"
    Completed = "completed"
    Cancelling = "cancelling"
    Cancelled = "cancelled"
    Failed = "failed"


TERMINAL_JOB_STATUSES = {JobStatus.Completed, JobStatus.Cancelled, JobStatus.Failed}
PENDING_JOB_STATUSES = set(JobStatus) - TERMINAL_JOB_STATUSES


class Job(UUIDModel):
    project = models.ForeignKey("Project", on_delete=models.CASCADE, related_name="jobs+")
    project_version = models.ForeignKey(
        "ProjectVersion", on_delete=models.CASCADE, related_name="jobs+"
    )
    deployment = models.ForeignKey(
        "Deployment", on_delete=models.SET_NULL, related_name="jobs+", null=True, blank=True
    )
    worker = models.ForeignKey("Worker", on_delete=models.SET_NULL, null=True, blank=True)
    parent = models.ForeignKey("Job", on_delete=models.CASCADE, related_name="children", null=True)
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    started_at = models.DateTimeField(null=True, blank=True)
    terminated_at = models.DateTimeField(null=True, blank=True)
    type = TextChoicesField(JobType)
    status = TextChoicesField(JobStatus)
    # Job-specific data
    build_candidate = models.ForeignKey(
        "BuildCandidate",
        null=True,
        blank=True,
        on_delete=models.SET_NULL,
        related_name="executions+",
    )
    symbol = models.ForeignKey("Statement", null=True, blank=True, on_delete=models.SET_NULL)

    def __str__(self):
        return f"{self.type} {self.id} ({self.status})"

    def __repr__(self):
        return f"<Job {self}>"

    class Meta:
        ordering = ["-updated_at"]
