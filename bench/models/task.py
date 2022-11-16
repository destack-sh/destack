from django.db import models

from bench.models import TaggableMixin
from bench.models.utils import MAX_NAME_LENGTH, UUIDModel


class TaskManager(models.Manager):
    pass


class Task(TaggableMixin, UUIDModel):
    """
    A task describes the interface and desired behaviour of an instruction.

    A task may have sub-tasks, forming a task tree.
    Sub-tasks define smaller tasks which are composed or represented by the parent task.
    """

    name = models.CharField(max_length=MAX_NAME_LENGTH)
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    parent = models.ForeignKey("Task", on_delete=models.CASCADE, null=True, related_name="children")

    schema = models.JSONField()
    # explanations from/to Explanation
    # examples from/to Example
    # expectations from/to Expectation
    template_implementation = models.ForeignKey(
        "Instruction", on_delete=models.CASCADE, null=True, related_name="templates"
    )
    # implementations from/to Instruction

    objects = TaskManager()

    def __str__(self):
        return f"{self.name}.task@{self.id.hex}"


class Explanation(UUIDModel):
    """
    A task explanation is a declarative (text-based) description of a task's expected behavior.
    """

    task = models.ForeignKey("Task", on_delete=models.CASCADE, related_name="explanations")
    dataset = models.ForeignKey("Dataset", on_delete=models.RESTRICT, related_name="explanations")
    dataset_view = models.ForeignKey(
        "DatasetView", on_delete=models.RESTRICT, null=True, related_name="explanations"
    )


class Example(UUIDModel):
    """
    A task example is an imperative (text-based) description of a task's expected behavior.
    """

    task = models.ForeignKey("Task", on_delete=models.CASCADE, related_name="examples")
    dataset = models.ForeignKey("Dataset", on_delete=models.RESTRICT, related_name="examples")
    dataset_view = models.ForeignKey(
        "DatasetView", on_delete=models.RESTRICT, null=True, related_name="examples"
    )


class ExpectationType(models.TextChoices):
    """
    The type of expectation defines its semantics.

    Invariants express expected stability of behavior respective to a task's input.
    Variants express expected variability of behavior respective to a task's input.
    Verifications express expected correctness of behavior respective to a task's output.
    """

    INVARIANCE = "invariance"
    VARIANCE = "variance"
    VERIFICATION = "verification"


class Expectation(UUIDModel):
    """
    A task expectation is an imperative (instruction-based) specification of a task's expected behavior.
    """

    type = models.CharField(max_length=64, choices=ExpectationType.choices)
    task = models.ForeignKey("Task", on_delete=models.CASCADE, related_name="expectations")
    instruction = models.ForeignKey(
        "Instruction", on_delete=models.RESTRICT, related_name="expectations"
    )
