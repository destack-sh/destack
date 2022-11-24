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
    index = models.IntegerField(default=0)

    schema = models.JSONField()
    # expectations from/to Expectation
    template_implementation = models.ForeignKey(
        "Instruction", on_delete=models.CASCADE, null=True, related_name="templates"
    )
    # implementations from/to Instruction

    objects: TaskManager = TaskManager()

    def __str__(self):
        return f"{self.name}.task@{self.id.hex}"


class Expectation(UUIDModel):
    """
    A task expectation specifies a task's expected behavior.
    It can instruct or explain the context and relations of expected behavior.

    The main text of an expectation is its description and instructions & examples are statements.
    """

    task = models.ForeignKey("Task", on_delete=models.CASCADE, related_name="expectations")
    index = models.IntegerField()
    description = models.TextField()
    instructions = models.ManyToManyField("Instruction", related_name="expectations")
    examples_datasets = models.ManyToManyField("Dataset", related_name="expectations")

    def __str__(self):
        return (
            f"{self.task}.expectations[{self.index}](description={self.description})@{self.id.hex}"
        )

    class Meta:
        ordering = ["index"]
        constraints = [
            models.UniqueConstraint(
                fields=["task", "index"], name="bench_expectation_task_index_ak"
            ),
        ]
