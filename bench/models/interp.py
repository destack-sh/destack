import uuid
from typing import TYPE_CHECKING, Optional, Union

from django.db import models

from bench.models.utils import ModuleNode, UUIDModel

if TYPE_CHECKING:
    from bench.models import File, Statement


class ResolvedField(UUIDModel, ModuleNode):
    """A field that has been resolved to a statement."""

    project_version = models.ForeignKey(
        "ProjectVersion", on_delete=models.CASCADE, related_name="+"
    )
    statement = models.ForeignKey(
        "Statement", on_delete=models.CASCADE, related_name="resolved_fields"
    )
    field = models.ForeignKey("Field", on_delete=models.CASCADE, related_name="+")

    @property
    def parent_id(self) -> Optional[uuid.UUID]:
        return self.statement_id


class IssueKind(models.TextChoices):
    ERROR = "error"
    WARNING = "warning"
    SUGGESTION = "suggestion"


class Issue(UUIDModel, ModuleNode):
    """An error/warning/... about a part of a project."""

    project_version = models.ForeignKey(
        "ProjectVersion", on_delete=models.CASCADE, related_name="issues"
    )
    parent_file = models.ForeignKey(
        "File", on_delete=models.CASCADE, related_name="issues", null=True
    )
    parent_statement = models.ForeignKey(
        "Statement", on_delete=models.CASCADE, related_name="issues", null=True
    )
    kind = models.CharField(max_length=32, choices=IssueKind.choices)
    type = models.CharField(max_length=64)
    message = models.CharField(max_length=512, null=True)

    def __str__(self):
        return f"{self.parent_statement or self.parent_file or self.project_version} {self.type} {self.message}"

    def __repr__(self):
        return f"<Issue {self}>"

    def parent(self) -> Union["Statement", "File"]:
        if self.parent_statement_id is not None:
            return self.parent_statement
        else:
            return self.parent_file

    @property
    def parent_id(self) -> Optional[uuid.UUID]:
        return self.parent_statement_id or self.parent_file_id
