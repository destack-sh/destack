import abc
import enum
import uuid
from dataclasses import dataclass
from typing import TYPE_CHECKING, Optional, Union
from uuid import UUID

from bench.bench.const import InterpScope, StatementPath, statement_path_as_str

if TYPE_CHECKING:
    from bench.bench.type import File, Statement, Symbol


class IssueKind(enum.StrEnum):
    ERROR = "error"
    WARNING = "warning"
    SUGGESTION = "suggestion"


class IssueType(enum.StrEnum):
    # errors
    INTERNAL = "INTERNAL"
    UNKNOWN_IMPORT_SOURCE = "UNKNOWN_IMPORT_SOURCE"
    MISSING_REFERENCE = "MISSING_REFERENCE"
    AMBIGUOUS_REQUIREMENT = "AMBIGUOUS_REQUIREMENT"
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
    IssueType.MISSING_REFERENCE.value: "missing reference {reference}",
    IssueType.AMBIGUOUS_REQUIREMENT.value: "multiple requirements for {path}",
    IssueType.CIRCULAR_ANCESTRY.value: "circular ancestry via {path}",
    IssueType.CIRCULAR_UNION.value: "circular union via {path}",
    IssueType.MISMATCHED_UNION.value: "mismatched union at {node} vs {other}",
    # warnings
    IssueType.AMBIGUOUS_DEFINITION.value: "multiple definitions for {path}",
}

_ISSUE_KIND_BY_TYPE = {
    # errors
    IssueType.INTERNAL.value: IssueKind.ERROR,
    IssueType.UNKNOWN_IMPORT_SOURCE.value: IssueKind.ERROR,
    IssueType.MISSING_REFERENCE.value: IssueKind.ERROR,
    IssueType.AMBIGUOUS_REQUIREMENT.value: IssueKind.ERROR,
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
    scope: Optional[InterpScope] = None
    subject: Union["Symbol", "Statement", "File", None] = None

    def __init__(
        self, type: IssueType, subject: Union["Symbol", "Statement", "File", None], **kwargs
    ):
        from bench.bench.type import File, Statement, Symbol

        # auto convert kwargs
        for key, value in kwargs.items():
            if isinstance(value, (Symbol, Statement, File)):
                kwargs[key] = value.name
            if isinstance(value, StatementPath):
                kwargs[key] = statement_path_as_str(value)

        self.type = type
        self.subject = subject
        if isinstance(subject, File):
            self.scope = InterpScope.FILE
            self.subject = subject
        elif isinstance(subject, (Statement, Symbol)):
            self.scope = InterpScope.STATEMENT
            self.subject = subject

        self.message = type.description.format(**kwargs)
        self.kind = _ISSUE_KIND_BY_TYPE[type]
        # generate id if not provided
        if "id" not in kwargs:
            self.id = uuid.uuid5(subject.id, type.value + self.message)
        else:
            self.id = kwargs.pop("id")

    @property
    def statement_id(self) -> UUID | None:
        if self.scope == InterpScope.STATEMENT:
            return self.subject.id
        return None

    @property
    def file_id(self) -> UUID | None:
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
        subject: Union["Symbol", "Statement", "File", None],
        type: IssueType,
        **kwargs,
    ):
        pass
