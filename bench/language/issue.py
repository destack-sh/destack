import abc
import enum
import uuid
from dataclasses import dataclass
from typing import TYPE_CHECKING, Optional, Union
from uuid import UUID

if TYPE_CHECKING:
    from bench.language.core import File, Statement


class IssueKind(enum.StrEnum):
    Error = "Error"
    Warning = "Warning"
    Suggestion = "Suggestion"


class IssueType(enum.StrEnum):
    # errors
    INTERNAL = "INTERNAL"
    UNKNOWN_IMPORT_SOURCE = "UNKNOWN_IMPORT_SOURCE"
    MISSING_REFERENCE = "MISSING_REFERENCE"
    CIRCULAR_ANCESTRY = "CIRCULAR_ANCESTRY"
    CIRCULAR_UNION = "CIRCULAR_UNION"
    MISMATCHED_UNION = "MISMATCHED_UNION"
    # warnings
    CODE_NOT_EXPORTABLE = "CODE_NOT_EXPORTABLE"
    CODE_NOT_CACHEABLE = "CODE_NOT_CACHEABLE"
    CODE_REFERENCE_NOT_EXPORTED = "CODE_REFERENCE_NOT_EXPORTED"
    AMBIGUOUS_DEFINITION = "AMBIGUOUS_DEFINITION"
    UNCLEAR_INTENT = "UNCLEAR_INTENT"

    @property
    def text(self):
        return _ISSUE_MESSAGES[self.value]


# separate from enum so that it's a simple StrEnum
_ISSUE_MESSAGES = {
    # errors
    IssueType.INTERNAL.value: "Internal error",
    IssueType.MISSING_REFERENCE.value: "missing reference at {path}",
    IssueType.CIRCULAR_ANCESTRY.value: "circular ancestry via {path}",
    IssueType.CIRCULAR_UNION.value: "circular union via {path}",
    IssueType.MISMATCHED_UNION.value: "mismatched union at {subject} vs {other}",
    # warnings
    IssueType.CODE_NOT_EXPORTABLE.value: "{subject} is not exportable",
    IssueType.CODE_REFERENCE_NOT_EXPORTED.value: "{path} is not exported",
    IssueType.AMBIGUOUS_DEFINITION.value: "multiple definitions for {path}",
    IssueType.UNCLEAR_INTENT.value: "unclear intent: {reason}",
}

_ISSUE_KIND_BY_TYPE = {
    # errors
    IssueType.INTERNAL.value: IssueKind.Error,
    IssueType.UNKNOWN_IMPORT_SOURCE.value: IssueKind.Error,
    IssueType.MISSING_REFERENCE.value: IssueKind.Error,
    IssueType.CIRCULAR_ANCESTRY.value: IssueKind.Error,
    IssueType.CIRCULAR_UNION.value: IssueKind.Error,
    IssueType.MISMATCHED_UNION.value: IssueKind.Error,
    # warnings
    IssueType.AMBIGUOUS_DEFINITION: IssueKind.Warning,
    IssueType.CODE_NOT_EXPORTABLE: IssueKind.Warning,
    IssueType.CODE_REFERENCE_NOT_EXPORTED: IssueKind.Warning,
    IssueType.UNCLEAR_INTENT: IssueKind.Warning,
}


class BenchError(ValueError):
    def __init__(self, issue: "Issue", **kwargs):
        super().__init__(issue.message.format(**kwargs))
        self.issue = issue


@dataclass
class Issue:
    id: Optional[UUID]
    ck: UUID
    kind: IssueKind
    type: IssueType
    message: str
    subject: Union["Statement", "Statement", "File", None] = None

    def __init__(
        self, type: IssueType, subject: Union["Statement", "Statement", "File", None], **kwargs
    ):
        from bench.language.core import File, Statement, StatementPath, statement_path_as_str

        # auto convert kwargs
        for key, value in kwargs.items():
            if isinstance(value, (Statement, Statement, File)):
                kwargs[key] = value.name
            if isinstance(value, StatementPath):
                kwargs[key] = statement_path_as_str(value)

        self.type = type
        self.subject = subject
        if isinstance(subject, File):
            self.subject = subject
        elif isinstance(subject, (Statement, Statement)):
            self.subject = subject

        if "subject" in type.text:
            kwargs["subject"] = self.subject
        self.message = type.text.format(**kwargs)
        self.kind = _ISSUE_KIND_BY_TYPE[type]
        # generate id if not provided
        if "id" not in kwargs:
            self.id = uuid.uuid5(subject.id, type.value + self.message)
        else:
            self.id = kwargs.pop("id")
        self.ck = self.id

    def __str__(self):
        return f"{self.subject} {self.kind}: {self.type} {self.message}"

    def __repr__(self):
        return f"<Issue {self}>"

    @property
    def parent_id(self) -> UUID | None:
        return self.subject.id if self.subject is not None else None

    @property
    def subject_id(self) -> UUID | None:
        return self.subject.id if self.subject is not None else None

    def to_error(self) -> BenchError:
        return BenchError(self)


class IssueHandler(abc.ABC):
    def __call__(
        self,
        issue: "Issue" = None,
        *,
        subject: Union["Statement", "Statement", "File", None],
        type: IssueType,
        **kwargs,
    ):
        pass
