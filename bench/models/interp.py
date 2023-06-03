from django.db import models

from bench.models.utils import UUIDModel


class IssueKind(models.TextChoices):
    ERROR = "error"
    WARNING = "warning"
    SUGGESTION = "suggestion"


class Issue(UUIDModel):
    """An error/warning/... about a part of a project."""

    project_version = models.ForeignKey(
        "ProjectVersion", on_delete=models.CASCADE, related_name="issues"
    )
    kind = models.CharField(max_length=32, choices=IssueKind.choices)
    statement = models.ForeignKey(
        "Statement", on_delete=models.CASCADE, related_name="issues", null=True
    )
    type = models.CharField(max_length=64)
    message = models.CharField(max_length=512, null=True)
