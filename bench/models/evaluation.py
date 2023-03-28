from django.db import models
from django_choices_field import TextChoicesField

from bench.models.utils import UUIDModel


class EvaluationKind(models.TextChoices):
    EVALUATION = "evaluation"
    LINT = "lint"


class EvaluationResult(UUIDModel):
    """
    A retained result from a Bench build/evaluation.
    """

    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    kind = TextChoicesField(EvaluationKind)
    project = models.ForeignKey("Project", on_delete=models.CASCADE, related_name="+")
    project_version = models.ForeignKey(
        "ProjectVersion", on_delete=models.CASCADE, related_name="+"
    )
    job = models.ForeignKey("Job", on_delete=models.CASCADE, related_name="+", null=True)
    file = models.ForeignKey("File", on_delete=models.CASCADE, related_name="+", null=True)
    build = models.ForeignKey("Statement", on_delete=models.CASCADE, related_name="+", null=True)
    statement = models.ForeignKey(
        "Statement", on_delete=models.CASCADE, related_name="+", null=True
    )
    record = models.ForeignKey(
        "DatasetRecord", on_delete=models.CASCADE, related_name="+", null=True
    )
    type_node = models.ForeignKey(
        "SimpleTypeNode", on_delete=models.CASCADE, related_name="+", null=True
    )
    self_metrics = models.JSONField(null=True, blank=True)
    aggregated_metrics = models.JSONField()
