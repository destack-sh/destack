from __future__ import annotations

import traceback
from contextlib import asynccontextmanager, contextmanager
from datetime import datetime, timezone
from typing import Optional, TypeVar

from asgiref.sync import sync_to_async
from django.db import models
from django.db.models import QuerySet

from bench.models.utils import UUIDModel


class ExecutionType(models.TextChoices):
    INSTRUCTION = "instruction", "Instruction"
    MODEL = "model", "Model"
    COMPILATION = "compilation", "Compilation"


ExecutionT = TypeVar("ExecutionT")


class ExecutionManager(models.Manager):
    def __init__(self, default_type: Optional[str] = None):
        super().__init__()
        self.default_type = default_type

    def get_queryset(self) -> QuerySet[Execution]:
        if self.default_type is not None:
            return super().get_queryset().filter(type=self.default_type)
        else:
            return super().get_queryset()

    def create(self, **kwargs):
        return super().create(type=self.default_type, **kwargs)


class Execution(UUIDModel):
    """
    The execution of some executable unit, like an instruction or model.

    An execution may be hierarchically nested inside other executions via the 'parent' field.
    """

    class Status(models.TextChoices):
        Created = "created"
        Scheduled = "scheduled"
        Queued = "queued"
        Running = "running"
        Aborting = "aborting"
        # terminal statuses
        Aborted = "aborted"
        Failed = "failed"
        Completed = "completed"

    TERMINAL_STATUSES = {Status.Aborted, Status.Failed, Status.Completed}
    PENDING_STATUSES = set(Status) - TERMINAL_STATUSES

    type = models.CharField(max_length=64, choices=ExecutionType.choices)
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    started_at = models.DateTimeField(
        blank=True, null=True, help_text="Time of transition to RUNNING status."
    )
    terminated_at = models.DateTimeField(
        blank=True, null=True, help_text="Time of transition to a terminal status."
    )
    status = models.CharField(max_length=32, choices=Status.choices, default=Status.Created)
    metadata = models.JSONField(null=True, blank=True)

    parent = models.ForeignKey(
        "Execution", on_delete=models.CASCADE, null=True, blank=True, related_name="children"
    )

    # relation to executable units
    instruction = models.ForeignKey(
        "Instruction",
        null=True,
        blank=True,
        on_delete=models.SET_NULL,
        related_name="executions",
    )
    model = models.ForeignKey(
        "Model", null=True, blank=True, on_delete=models.SET_NULL, related_name="executions"
    )
    compilation = models.ForeignKey(
        "Compilation",
        null=True,
        blank=True,
        on_delete=models.SET_NULL,
        related_name="executions",
    )

    # relation to organizational units
    organization: models.ForeignKey = models.ForeignKey(
        "Organization", on_delete=models.CASCADE, related_name="executions+"
    )
    project: models.ForeignKey = models.ForeignKey(
        "Project", on_delete=models.CASCADE, null=True, related_name="executions+"
    )

    objects = ExecutionManager()

    def _set_transition_metadata(
        self, status: Execution.Status, transition_metadata: Optional[dict]
    ):
        if transition_metadata is None:
            return
        if self.metadata is None:
            self.metadata = {}
        self.metadata[status.value] = transition_metadata

    def update_status(self, status: Execution.Status, transition_metadata: Optional[dict] = None):
        self.status = status
        self._set_transition_metadata(status, transition_metadata)
        self.save()

    def start(
        self, status: Execution.Status = Status.Running, transition_metadata: Optional[dict] = None
    ):
        """
        Marks this execution as started in the given status
        """
        self.started_at = datetime.utcnow().astimezone(tz=timezone.utc)
        self.status = status
        self._set_transition_metadata(status, transition_metadata)
        self.save()

    def terminate(
        self,
        status: Execution.Status = Status.Completed,
        transition_metadata: Optional[dict] = None,
    ):
        """
        Marks this execution as terminated in the given status
        """
        self.terminated_at = datetime.utcnow().astimezone(tz=timezone.utc)
        self.status = status
        self._set_transition_metadata(status, transition_metadata)
        self.save()

    @contextmanager
    def capture(self, start: bool = True, start_metadata: Optional[dict] = None):
        try:
            if start:
                self.start(transition_metadata=start_metadata)
            yield
            self.terminate()
        except Exception as e:
            stacktrace = traceback.format_stack()
            self.terminate(
                status=Execution.Status.Failed,
                transition_metadata={"error": str(e), "stacktrace": stacktrace},
            )
            raise

    @asynccontextmanager
    async def acapture(self, start: bool = True, start_metadata: Optional[dict] = None):
        try:
            if start:
                sync_to_async(self.start)(transition_metadata=start_metadata)
            yield
            sync_to_async(self.terminate)()
        except Exception as e:
            stacktrace = traceback.format_stack()
            sync_to_async(self.terminate)(
                status=Execution.Status.Failed,
                transition_metadata={"error": str(e), "stacktrace": stacktrace},
            )
            raise
