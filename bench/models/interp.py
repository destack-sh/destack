from django.db import models

from bench.models.utils import UUIDModel


class InterpScope(models.TextChoices):
    MODULE = "module"
    FILE = "file"
    STATEMENT = "statement"


class ResolvedField(models.Model):
    """A field that has been resolved to a statement."""

    id = models.IntegerField(primary_key=True)
    project_version = models.ForeignKey(
        "ProjectVersion", on_delete=models.CASCADE, related_name="+"
    )
    statement = models.ForeignKey(
        "Statement", on_delete=models.CASCADE, related_name="resolved_fields+"
    )
    field = models.ForeignKey("Field", on_delete=models.CASCADE, related_name="+")


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
    scope = models.CharField(max_length=32, choices=InterpScope.choices)
    file = models.ForeignKey("File", on_delete=models.CASCADE, related_name="issues", null=True)
    statement = models.ForeignKey(
        "Statement", on_delete=models.CASCADE, related_name="issues", null=True
    )
    type = models.CharField(max_length=64)
    message = models.CharField(max_length=512, null=True)
