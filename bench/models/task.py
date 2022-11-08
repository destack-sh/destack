from django.db import models

from bench.models import TaggableMixin
from bench.models.utils import MAX_NAME_LENGTH, UUIDModel


class TaskManager(models.Manager):
    pass


class Task(TaggableMixin, UUIDModel):
    """
    A task describes the interface and desired behaviour of an instruction flow.
    """

    name = models.CharField(max_length=MAX_NAME_LENGTH)
    description = models.CharField(max_length=512)
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)

    schema = models.JSONField()
    definitions = models.ManyToManyField("Dataset", related_name="tasks")
    examples = models.ManyToManyField("Dataset", related_name="tasks")
    expectations = models.ManyToManyField("Flow", related_name="tasks")
    # implementations from/to FlowInstruction

    project = models.ForeignKey("Project", on_delete=models.CASCADE, related_name="tasks")

    objects = TaskManager()
