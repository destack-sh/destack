from __future__ import annotations

import traceback
from contextlib import contextmanager
from datetime import datetime, timezone
from typing import Optional, TypeVar

from django.db import models
from django.db.models import QuerySet

from bench.models.utils import UUIDModel

FLOW_EXECUTION_TYPE = "flow"
FLOW_INSTRUCTION_EXECUTION_TYPE = "flow_instruction"
MODEL_EXECUTION_TYPE = "model"
JOB_EXECUTION_TYPE = "job"

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
    The execution of some executable unit, like a flow or instruction.

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

    type = models.CharField(max_length=64)
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
    flow = models.ForeignKey(
        "Flow", null=True, blank=True, on_delete=models.SET_NULL, related_name="executions"
    )
    flow_instruction = models.ForeignKey(
        "FlowInstruction",
        null=True,
        blank=True,
        on_delete=models.SET_NULL,
        related_name="executions",
    )
    model = models.ForeignKey(
        "Model", null=True, blank=True, on_delete=models.SET_NULL, related_name="executions"
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


class FlowExecution(Execution):
    """
    The execution of an entire Flow.
    """

    objects = ExecutionManager(default_type=FLOW_EXECUTION_TYPE)

    class Meta:
        proxy = True


class FlowInstructionExecution(Execution):
    """
    The parameterised execution of a specific node in a Flow.
    """

    objects = ExecutionManager(default_type=FLOW_INSTRUCTION_EXECUTION_TYPE)

    class Meta:
        proxy = True


class ModelExecution(Execution):
    """
    The execution of an individual model artifact (also called a 'prediction').
    """

    objects = ExecutionManager(default_type=MODEL_EXECUTION_TYPE)

    class Meta:
        proxy = True
