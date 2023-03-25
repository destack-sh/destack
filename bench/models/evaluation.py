from django.db import models

from bench.models.utils import UUIDModel


class EvaluationResult(UUIDModel):
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    project = models.ForeignKey("Project", on_delete=models.CASCADE, related_name="+")
    project_version = models.ForeignKey(
        "ProjectVersion", on_delete=models.CASCADE, related_name="+"
    )
    job = models.ForeignKey("Job", on_delete=models.CASCADE, related_name="+", null=True)
    build = models.ForeignKey("Statement", on_delete=models.CASCADE, related_name="+")
    system = models.ForeignKey("Statement", on_delete=models.CASCADE, related_name="+")
    metrics = models.JSONField()
    parent = models.ForeignKey(
        "EvaluationResult", on_delete=models.CASCADE, related_name="children+", null=True
    )
