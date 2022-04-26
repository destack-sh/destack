from django.db import models

from bench.models.utils import UUIDModel


class ExecutionManager(models.Manager):
    pass


class Execution(UUIDModel):
    """
    The execution of some executable unit, like a function or a flow.
    """

    class State(models.TextChoices):
        Created = "CREATED"
        Scheduled = "SCHEDULED"
        Queued = "QUEUED"
        Running = "RUNNING"
        Succeeded = "SUCCEEDED"
        Failed = "FAILED"
        Aborting = "ABORTING"
        Aborted = "ABORTED"

    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    started_at = models.DateTimeField(
        blank=True, null=True, help_text="Time of transition to RUNNING state."
    )
    completed_at = models.DateTimeField(
        blank=True, null=True, help_text="Time of transition to a terminal state."
    )
    state = models.CharField(
        max_length=256, choices=State.choices, default=State.Created
    )

    objects = ExecutionManager()


class FunctionExecution(Execution):
    function = models.ForeignKey("Function", on_delete=models.CASCADE)


class FlowExecution(Execution):
    flow = models.ForeignKey("Flow", on_delete=models.CASCADE)


class ModelExecution(Execution):
    model = models.ForeignKey("Model", on_delete=models.CASCADE)
