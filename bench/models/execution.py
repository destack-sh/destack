from __future__ import annotations

from django.db import models

from bench.models.utils import UUIDModel


class ExecutionManager(models.Manager):
    pass


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
    metadata = models.JSONField()

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


class FlowExecution(Execution):
    """
    The execution of an entire Flow.
    """

    class Meta:
        proxy = True


class FlowNodeExecution(Execution):
    """
    The parameterised execution of a specific node in a Flow.
    """

    class Meta:
        proxy = True


class ModelExecution(Execution):
    """
    The execution of an individual model artifact (also called a 'prediction').
    """

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
    # optional FlowArtifactEdge reference when used for disambiguation
    flow_artifact_edge = models.ForeignKey(
        "FlowArtifactEdge", blank=True, null=True, on_delete=models.CASCADE
    )

    artifact = models.ForeignKey("ArtifactVersion", on_delete=models.CASCADE)
    # regular ArtifactView relation where appropriate
    view = models.ForeignKey("ArtifactView", on_delete=models.CASCADE, null=True, blank=True)
    # inlined ArtifactView if we don't want/need a full ArtifactView
    view_type = models.CharField(null=True, blank=True, max_length=64)
    view_data = models.JSONField(null=True, blank=True)
