import abc
import enum
import uuid
from dataclasses import dataclass
from typing import TYPE_CHECKING, Optional, Union
from uuid import UUID

if TYPE_CHECKING:
    from bench.language.core import File, InterpScope, Statement, statement_path_as_str


class IssueKind(enum.StrEnum):
    ERROR = "error"
    WARNING = "warning"
    SUGGESTION = "suggestion"


class IssueType(enum.StrEnum):
    # errors
    INTERNAL = "INTERNAL"
    UNKNOWN_IMPORT_SOURCE = "UNKNOWN_IMPORT_SOURCE"
    MISSING_REFERENCE = "MISSING_REFERENCE"
    CIRCULAR_ANCESTRY = "CIRCULAR_ANCESTRY"
    CIRCULAR_UNION = "CIRCULAR_UNION"
    MISMATCHED_UNION = "MISMATCHED_UNION"
    # warnings
    AMBIGUOUS_DEFINITION = "AMBIGUOUS_DEFINITION"

    @property
    def description(self):
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
    IssueType.AMBIGUOUS_DEFINITION.value: "multiple definitions for {path}",
}

_ISSUE_KIND_BY_TYPE = {
    # errors
    IssueType.INTERNAL.value: IssueKind.ERROR,
    IssueType.UNKNOWN_IMPORT_SOURCE.value: IssueKind.ERROR,
    IssueType.MISSING_REFERENCE.value: IssueKind.ERROR,
    IssueType.CIRCULAR_ANCESTRY.value: IssueKind.ERROR,
    IssueType.CIRCULAR_UNION.value: IssueKind.ERROR,
    IssueType.MISMATCHED_UNION.value: IssueKind.ERROR,
    # warnings
    IssueType.AMBIGUOUS_DEFINITION: IssueKind.WARNING,
}


class BenchError(ValueError):
    def __init__(self, issue: "Issue", **kwargs):
        super().__init__(issue.message.format(**kwargs))
        self.issue = issue


@dataclass
class Issue:
    id: Optional[UUID]
    kind: IssueKind
    type: IssueType
    message: str
    scope: Optional["InterpScope"] = None
    subject: Union["Statement", "Statement", "File", None] = None

    def __init__(
        self, type: IssueType, subject: Union["Statement", "Statement", "File", None], **kwargs
    ):
        from bench.language.core import File, InterpScope, Statement, StatementPath

        # auto convert kwargs
        for key, value in kwargs.items():
            if isinstance(value, (Statement, Statement, File)):
                kwargs[key] = value.name
            if isinstance(value, StatementPath):
                kwargs[key] = statement_path_as_str(value)

        self.type = type
        self.subject = subject
        if isinstance(subject, File):
            self.scope = InterpScope.FILE
            self.subject = subject
        elif isinstance(subject, (Statement, Statement)):
            self.scope = InterpScope.STATEMENT
            self.subject = subject

        if "subject" in type.description:
            kwargs["subject"] = self.subject
        self.message = type.description.format(**kwargs)
        self.kind = _ISSUE_KIND_BY_TYPE[type]
        # generate id if not provided
        if "id" not in kwargs:
            self.id = uuid.uuid5(subject.id, type.value + self.message)
        else:
            self.id = kwargs.pop("id")

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

    @property
    def statement_id(self) -> UUID | None:
        from bench.language.core import InterpScope

        if self.scope == InterpScope.STATEMENT:
            return self.subject.id
        return None

    @property
    def file_id(self) -> UUID | None:
        from bench.language.core import InterpScope

        if self.scope == InterpScope.FILE:
            return self.subject.id
        if self.scope == InterpScope.STATEMENT:
            return self.subject.file.id
        return None

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
