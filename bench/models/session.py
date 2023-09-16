from __future__ import annotations

from typing import Optional
from uuid import UUID

from django.db import models
from django.db.models import Model
from django.db.models.expressions import RawSQL
from strawberry_django.descriptors import model_property

from bench.language.const import RunStatus, TriggerType
from bench.models.utils import UUIDTModel, get_choices


class HasTriggeredBy(Model):
    trigger_type = models.CharField(
        max_length=32, choices=get_choices(TriggerType), null=True, blank=True
    )
    trigger_user = models.ForeignKey("User", null=True, blank=True, on_delete=models.SET_NULL)
    trigger_access_token = models.ForeignKey(
        "AccessToken", null=True, blank=True, on_delete=models.SET_NULL
    )
    trigger = models.ForeignKey("Trigger", null=True, blank=True, on_delete=models.SET_NULL)

    @property
    def trigger_id(self):
        return self.trigger_id or self.trigger_access_token_id or self.trigger_user_id

    class Meta:
        abstract = True


class Session(UUIDTModel, HasTriggeredBy):
    project_version = models.ForeignKey("ProjectVersion", on_delete=models.CASCADE)
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    opened_at = models.DateTimeField(null=True, blank=True)
    closed_at = models.DateTimeField(null=True, blank=True)
    metadata = models.JSONField(null=True, blank=True)


class RunManager(models.Manager):
    def get_descendants(self, run_ids: list[UUID]):
        """Gets descendants of runs with given ids (including the runs themselves)"""
        query = """
          WITH RECURSIVE descendants(id, parent_id) AS (
              SELECT id, parent_id
              FROM bench_run
              WHERE id = ANY(%s)
              UNION ALL
              SELECT bench_run.id, bench_run.parent_id
              FROM bench_run
              INNER JOIN descendants ON descendants.id = bench_run.parent_id
         )
         SELECT DISTINCT id
         FROM descendants
         """
        return Run._base_manager.filter(id__in=RawSQL(query, (run_ids,)))


class Run(UUIDTModel, HasTriggeredBy):
    project = models.ForeignKey(
        "Project", on_delete=models.CASCADE, null=True, blank=True, related_name="runs"
    )
    project_version = models.ForeignKey("ProjectVersion", on_delete=models.CASCADE)
    worker_node_id = models.CharField(max_length=64, null=True, blank=True)
    worker_process_id = models.CharField(max_length=64, null=True, blank=True)
    session = models.ForeignKey(
        "Session", on_delete=models.CASCADE, null=True, blank=True, related_name="runs"
    )
    root = models.ForeignKey(
        "Run", on_delete=models.CASCADE, null=True, blank=True, related_name="root_descendants"
    )
    parent = models.ForeignKey(
        "Run", on_delete=models.CASCADE, null=True, blank=True, related_name="children"
    )
    root_descendants: models.QuerySet[Run]  # noqa via Run.root
    children: models.QuerySet[Run]  # noqa via Run.parent

    status = models.CharField(max_length=32, choices=get_choices(RunStatus))
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    scheduled_at = models.DateTimeField(null=True, blank=True)
    started_at = models.DateTimeField(null=True, blank=True)
    terminated_at = models.DateTimeField(null=True, blank=True)

    runnable = models.ForeignKey("Statement", null=True, blank=True, on_delete=models.SET_NULL)
    runnable_type = models.CharField(max_length=64, null=True, blank=True)
    runnable_ck = models.UUIDField(null=True, blank=True)
    inputs = models.JSONField(null=True, blank=True)
    outputs = models.JSONField(null=True, blank=True)
    error = models.JSONField(null=True, blank=True)
    metadata = models.JSONField(null=True, blank=True)

    @model_property(only=["started_at", "terminated_at"])
    def duration(self) -> Optional[float]:
        if self.started_at and self.terminated_at:
            return (self.terminated_at - self.started_at).total_seconds()
        return None

    def __str__(self):
        root_str = f"root={self.root_id}" if self.root_id else ""
        parent_str = f"parent={self.parent_id}" if self.parent_id else ""
        return f"{self.id} {self.status} ({(root_str + ' ' + parent_str).strip()})"

    objects = RunManager()

    def descendants(self) -> models.QuerySet[Run]:
        if self.parent_id is None:
            return self.root_descendants.all()
        else:
            return Run.objects.get_descendants([self.id])

    class Meta:
        ordering = ["-created_at"]
        indexes = [
            models.Index(fields=["started_at"], name="run_started_at_idx"),
            models.Index(fields=["updated_at"], name="run_updated_at_idx"),
            models.Index(fields=["runnable_id"], name="run_runnable_id_idx"),
            models.Index(fields=["worker_node_id"], name="run_worker_node_idx"),
            models.Index(
                fields=["runnable_id", "project_version_id"], name="run_runnable_id_scoped_idx"
            ),
        ]


# LogEntry lives in OpenSearch only
