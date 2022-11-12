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
    description = models.CharField(max_length=512)
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    parent = models.ForeignKey("Task", on_delete=models.CASCADE, null=True, related_name="children")

    schema = models.JSONField()
    examples = models.ManyToManyField("Dataset", related_name="tasks+")
    expectations = models.ManyToManyField("Instruction", related_name="tasks+")
    # implementations from/to Instruction

    organization = models.ForeignKey("Organization", on_delete=models.CASCADE, related_name="tasks")
    project = models.ForeignKey("Project", on_delete=models.CASCADE, related_name="tasks")

    objects = TaskManager()

    def __str__(self):
        return f"{self.organization.slug}/{self.project.slug}/tasks/{self.name}@{self.id}"
