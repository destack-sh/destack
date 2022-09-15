from django.db import models

from bench.models import DbRecord, TaggableMixin


# TODO @Robustness: prevent Task creation without existing DbRecord in a Dataset.
#  if a task is a DbRecord without a dataset it will be GCed since it doesn't belong to any tree
class Task(TaggableMixin, DbRecord):
    """A task is an atomic unit of annotation work to be completed"""

    class Status(models.TextChoices):
        Created = "created"
        Assigned = "assigned"
        Completed = "completed"

    # record data (from DbRecord)
    # data = models.JSONField()
    # metadata = models.JSONField(null=True, blank=True)
    # task metadata
    status = models.CharField(max_length=32, choices=Status.choices, default=Status.Created)
    project = models.ForeignKey("Project", on_delete=models.CASCADE, related_name="tasks")
    batch = models.ForeignKey("Batch", on_delete=models.CASCADE, related_name="batches")
    assigned_user = models.ForeignKey(
        "bench.User", on_delete=models.SET_NULL, related_name="assigned_tasks", null=True
    )
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    completed_at = models.DateTimeField(auto_now=True)
