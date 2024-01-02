import uuid
from typing import TYPE_CHECKING, Optional, Union

from django.db import models

from bench.language.validation import MAX_NAME_LENGTH
from bench.models.utils import Node, UUIDModel

if TYPE_CHECKING:
    from bench.models import File, Statement


class ResolvedField(UUIDModel, Node):
    """A field that has been resolved to a statement."""

    bench_version = models.ForeignKey("BenchVersion", on_delete=models.CASCADE, related_name="+")
    statement = models.ForeignKey(
        "Statement", on_delete=models.CASCADE, related_name="resolved_fields"
    )
    order_key = models.CharField(max_length=MAX_NAME_LENGTH)
    field_ck = models.UUIDField()

    @property
    def parent_id(self) -> Optional[uuid.UUID]:
        return self.statement_id

    class Meta:
        managed = False


class IssueKind(models.TextChoices):
    Error = "Error"
    Warning = "Warning"
    Notice = "Notice"


class Issue(UUIDModel, Node):
    """An error/warning/... about a part of a bench."""

    bench_version = models.ForeignKey(
        "BenchVersion", on_delete=models.CASCADE, related_name="issues"
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
        return f"{self.parent_statement or self.parent_file or self.bench_version} {self.type} {self.message}"

    def __repr__(self):
        return f"<Issue {self}>"

    @property
    def parent(self) -> Union["Statement", "File"]:
        if self.parent_statement_id is not None:
            return self.parent_statement
        else:
            return self.parent_file

    @property
    def parent_id(self) -> Optional[uuid.UUID]:
        return self.parent_statement_id or self.parent_file_id

    class Meta:
        managed = False
