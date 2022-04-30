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

    objects = ExecutionManager()


class FlowExecution(Execution):
    flow = models.ForeignKey("Flow", on_delete=models.CASCADE)


class FlowNodeExecution(Execution):
    flow_node = models.ForeignKey("FlowNode", on_delete=models.CASCADE)
    config_arguments = models.JSONField()
    connected_artifacts = models.ManyToManyField(
        "ArtifactVersion", related_name="source_execution"
    )


class ModelExecution(Execution):
    model = models.ForeignKey("Model", on_delete=models.CASCADE)
