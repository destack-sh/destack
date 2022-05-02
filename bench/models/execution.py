from __future__ import annotations

from django.db import models

from bench.models.utils import UUIDModel


class ExecutionManager(models.Manager):
    pass


class Execution(UUIDModel):
    """
    The execution of some executable unit, like a function or a flow.
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

    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    started_at = models.DateTimeField(
        blank=True, null=True, help_text="Time of transition to RUNNING state."
    )
    terminated_at = models.DateTimeField(
        blank=True, null=True, help_text="Time of transition to a terminal state."
    )
    state = models.CharField(
        max_length=32, choices=State.choices, default=State.Created
    )
    parent = models.ForeignKey(
        "Execution", on_delete=models.CASCADE, related_name="children"
    )

    objects = ExecutionManager()


class FlowExecution(Execution):
    """
    The execution of an entire Flow.
    """

    flow = models.ForeignKey("Flow", on_delete=models.CASCADE)


class FlowNodeExecution(Execution):
    """
    The parameterised execution of a specific node in a Flow.
    """

    flow_node = models.ForeignKey("FlowNode", on_delete=models.CASCADE)
    config_arguments = models.JSONField()
    connected_artifacts = models.ManyToManyField(
        "ArtifactVersion",
        through="FlowNodeExecutionArtifactConnection",
        related_name="related_executions",
    )


class FlowNodeExecutionArtifactConnection(UUIDModel):
    """
    The runtime connection between an executed flow node and its related artifacts.

    We specify how the connection is made via the connection parameters.
    If the node is directly connected to an artifact, we specify the relevant 'edge'.
    If the execution refers to a part of the artifact, we specify the relevant 'view'.
    """

    class ConnectionType(models.TextChoices):
        Argument = "argument"
        Input = "input"
        Output = "output"

    execution = models.ForeignKey(FlowNodeExecution, on_delete=models.CASCADE)
    edge = models.ForeignKey("FlowArtifactEdge", null=True, on_delete=models.CASCADE)
    connection_type = models.CharField(max_length=32, choices=ConnectionType.choices)
    connection_name = models.CharField(max_length=64, null=True, blank=True)
    artifact = models.ForeignKey("ArtifactVersion", on_delete=models.CASCADE)
    view = models.ForeignKey(
        "ArtifactView", on_delete=models.CASCADE, null=True, blank=True
    )


class ModelExecution(Execution):
    """
    The execution of an individual model artifact (also called a 'prediction').
    """

    model = models.ForeignKey("ModelVersion", on_delete=models.CASCADE)
    input_artifact = models.ForeignKey(
        "ArtifactVersion", on_delete=models.CASCADE, null=True, related_name="+"
    )
    input_artifact_view_type = models.CharField(null=True, blank=True, max_length=64)
    input_artifact_view_metadata = models.JSONField(null=True, blank=True)
    output_artifact = models.ForeignKey(
        "ArtifactVersion", on_delete=models.CASCADE, null=True, related_name="+"
    )
    output_artifact_view_type = models.CharField(null=True, blank=True, max_length=64)
    output_artifact_view_metadata = models.JSONField(null=True, blank=True)
