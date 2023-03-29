from uuid import UUID

from django.db import models
from django_choices_field import TextChoicesField

from bench.models.utils import UUIDModel


class EvaluationKind(models.TextChoices):
    EVALUATION = "evaluation"
    LINT = "lint"


class EvaluationScope(models.TextChoices):
    INSTRUCTION = "node"
    BUILD = "build"
    MODULE = "module"


class EvaluationResult(UUIDModel):
    """
    A retained result from a Bench build/evaluation.
    """

    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    kind = TextChoicesField(EvaluationKind)
    scope = TextChoicesField(EvaluationScope)
    project = models.ForeignKey("Project", on_delete=models.CASCADE, related_name="+")
    project_version = models.ForeignKey(
        "ProjectVersion", on_delete=models.CASCADE, related_name="+"
    )
    job = models.ForeignKey("Job", on_delete=models.CASCADE, related_name="+", null=True)
    file = models.ForeignKey("File", on_delete=models.CASCADE, related_name="+", null=True)
    build = models.ForeignKey("Statement", on_delete=models.CASCADE, related_name="+", null=True)
    build_candidate = models.ForeignKey(
        "BuildCandidate",
        null=True,
        blank=True,
        on_delete=models.SET_NULL,
        related_name="executions+",
    )
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

    @property
    def system(self):
        return self.statement or self.build or self.type_node

    @property
    def system_id(self) -> UUID:
        return self.system.id if self.system else None

    def __str__(self):
        metrics_str = ", ".join(
            f"{k}: {self.aggregated_metrics[k]:0.02f}" for k, v in self.aggregated_metrics.items()
        )
        return f"{self.kind} {self.scope} {metrics_str}"

    def __repr__(self):
        return f"<EvaluationResult {self}>"

    class Meta:
        ordering = ["-created_at"]
