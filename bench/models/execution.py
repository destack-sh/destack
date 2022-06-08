from __future__ import annotations

import traceback
from contextlib import contextmanager
from datetime import datetime
from typing import Optional, TypeVar

from django.db import models
from django.db.models import QuerySet

from bench.models.utils import UUIDModel

FLOW_EXECUTION_TYPE = "flow"
FLOW_NODE_EXECUTION_TYPE = "flow_node"
MODEL_EXECUTION_TYPE = "model"
JOB_EXECUTION_TYPE = "job"

DEFAULT_CONNECTION_NAME = "main"

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
    The execution of some executable unit, like a function or a flow.

    An execution may be hierarchically nested inside other executions via the 'parent' field.
    """

    class State(models.TextChoices):
        Created = "created"
        Scheduled = "scheduled"
        Queued = "queued"
        Running = "running"
        Aborting = "aborting"
        # terminal states
        Aborted = "aborted"
        Failed = "failed"
        Completed = "completed"

    type = models.CharField(max_length=64)
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    started_at = models.DateTimeField(
        blank=True, null=True, help_text="Time of transition to RUNNING state."
    )
    terminated_at = models.DateTimeField(
        blank=True, null=True, help_text="Time of transition to a terminal state."
    )
    state = models.CharField(max_length=32, choices=State.choices, default=State.Created)
    metadata = models.JSONField(null=True, blank=True)

    parent = models.ForeignKey(
        "Execution", on_delete=models.CASCADE, null=True, blank=True, related_name="children"
    )
    # relation to executable units
    flow = models.ForeignKey(
        "FlowVersion", null=True, blank=True, on_delete=models.SET_NULL, related_name="executions"
    )
    flow_node = models.ForeignKey(
        "FlowNode", null=True, blank=True, on_delete=models.SET_NULL, related_name="executions"
    )
    model = models.ForeignKey(
        "ModelVersion", null=True, blank=True, on_delete=models.SET_NULL, related_name="executions"
    )

    objects = ExecutionManager()

    def _set_transition_metadata(self, state: Execution.State, transition_metadata: Optional[dict]):
        if transition_metadata is None:
            return
        if self.metadata is None:
            self.metadata = {}
        self.metadata[state.value] = transition_metadata

    def update_state(self, state: Execution.State, transition_metadata: Optional[dict] = None):
        self.state = state
        self._set_transition_metadata(state, transition_metadata)
        self.save()

    def start(
        self, state: Execution.State = State.Running, transition_metadata: Optional[dict] = None
    ):
        """
        Marks this execution as started in the given state
        """
        self.started_at = datetime.utcnow()
        self.state = state
        self._set_transition_metadata(state, transition_metadata)
        self.save()

    def terminate(
        self, state: Execution.State = State.Completed, transition_metadata: Optional[dict] = None
    ):
        """
        Marks this execution as terminated in the given state
        """
        self.terminated_at = datetime.utcnow()
        self.state = state
        self._set_transition_metadata(state, transition_metadata)
        self.save()

    @contextmanager
    def capture(self, start: bool = True):
        try:
            if start:
                self.start()
            yield
            self.terminate()
        except Exception as e:
            self.terminate(
                state=Execution.State.Failed,
                transition_metadata={"error": str(e), "stacktrace": traceback.format_stack()},
            )
            raise


class FlowExecution(Execution):
    """
    The execution of an entire Flow.
    """

    objects = ExecutionManager(default_type=FLOW_EXECUTION_TYPE)

    class Meta:
        proxy = True


class FlowNodeExecution(Execution):
    """
    The parameterised execution of a specific node in a Flow.
    """

    objects = ExecutionManager(default_type=FLOW_NODE_EXECUTION_TYPE)

    class Meta:
        proxy = True


class ModelExecution(Execution):
    """
    The execution of an individual model artifact (also called a 'prediction').
    """

    objects = ExecutionManager(default_type=MODEL_EXECUTION_TYPE)

    class Meta:
        proxy = True


class ExecutionArtifactConnection(UUIDModel):
    """
    The runtime connection between an execution and its related artifacts.

    We specify how the connection is made via the connection parameters.
    If the node is directly connected to an artifact, we specify the relevant 'edge'.
    If the execution refers to a part of the artifact, we specify the relevant 'view'.
    """

    class ConnectionType(models.TextChoices):
        Argument = "argument"
        Input = "input"
        Output = "output"

    execution = models.ForeignKey(
        Execution, on_delete=models.CASCADE, related_name="connected_artifacts"
    )
    connection_type = models.CharField(max_length=32, choices=ConnectionType.choices)
    connection_name = models.CharField(max_length=64, null=True, blank=True)
    # optional FlowArtifactEdge reference when defined or used for disambiguation
    flow_artifact_edge = models.ForeignKey(
        "FlowArtifactEdge", blank=True, null=True, on_delete=models.CASCADE
    )

    artifact = models.ForeignKey(
        "ArtifactVersion", on_delete=models.CASCADE, related_name="execution_connections"
    )
    # regular ArtifactView relation where appropriate
    view = models.ForeignKey(
        "ArtifactView",
        on_delete=models.CASCADE,
        null=True,
        blank=True,
        related_name="execution_connections",
    )
    # inlined ArtifactView if we don't want/need a full ArtifactView
    view_type = models.CharField(null=True, blank=True, max_length=64)
    view_data = models.JSONField(null=True, blank=True)
