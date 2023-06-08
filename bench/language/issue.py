import enum
from dataclasses import dataclass
from typing import TYPE_CHECKING, Optional, Union

from bench.language.const import InterpScope

if TYPE_CHECKING:
    from bench.language.type import File, Statement, Symbol


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

    def __new__(cls, value):
        obj = str.__new__(cls)
        obj._value_ = value
        return obj

    @property
    def description(self):
        return _ISSUE_MESSAGES[self]

    @property
    def id(self) -> int:
        return self.value[0]


# separate from enum so that it's a simple StrEnum
_ISSUE_MESSAGES = {
    # errors
    IssueType.INTERNAL: "Internal error",
    IssueType.MISSING_REFERENCE: "missing reference",
    IssueType.AMBIGUOUS_REQUIREMENT: "multiple requirements for {path}",
    IssueType.CIRCULAR_ANCESTRY: "circular ancestry via {path}",
    IssueType.CIRCULAR_UNION: "circular union via {path}",
    IssueType.MISMATCHED_UNION: "mismatched union at {node} vs {other}",
    # warnings
    IssueType.AMBIGUOUS_DEFINITION: "multiple definitions for {path}",
}

_ISSUE_KIND_BY_TYPE = {
    # errors
    IssueType.INTERNAL: IssueKind.ERROR,
    IssueType.UNKNOWN_IMPORT_SOURCE: IssueKind.ERROR,
    IssueType.MISSING_REFERENCE: IssueKind.ERROR,
    IssueType.AMBIGUOUS_REQUIREMENT: IssueKind.ERROR,
    IssueType.CIRCULAR_ANCESTRY: IssueKind.ERROR,
    IssueType.CIRCULAR_UNION: IssueKind.ERROR,
    IssueType.MISMATCHED_UNION: IssueKind.ERROR,
    # warnings
    IssueType.AMBIGUOUS_DEFINITION: IssueKind.ERROR,
}


class LanguageError(ValueError):
    def __init__(self, issue: "Issue", **kwargs):
        super().__init__(issue.message.format(**kwargs))
        self.issue = issue


@dataclass
class Issue:
    kind: IssueKind
    type: IssueType
    message: str
    scope: Optional[InterpScope] = None
    subject: Union["Symbol", "Statement", "File", None] = None

    def __init__(
        self, type: IssueType, subject: Union["Symbol", "Statement", "File", None], **kwargs
    ):
        from bench.language.type import File, Statement, Symbol

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

    def to_error(self) -> LanguageError:
        return LanguageError(self)


class IssueHandler:
    def __call__(
        self,
        issue: "Issue" = None,
        *,
        subject: Union["Symbol", "Statement", "File", None],
        type: IssueType,
        **kwargs,
    ):
        pass


class IssueRaiser(IssueHandler):
    def __init__(self, kinds: list[IssueKind] = None):
        self.kinds = kinds

    def __call__(
        self,
        issue: Issue = None,
        *,
        type: IssueType,
        subject: Union["Symbol", "Statement", "File", None],
        **kwargs,
    ):
        issue = issue or Issue(type, subject, **kwargs)
        if self.kinds is None or issue.kind in self.kinds:
            raise issue.to_error()


class IssueCollector(IssueHandler):
    def __init__(self, on_issue: IssueHandler | None = None):
        self.on_issue = on_issue
        self.issues: list[Issue] = []

    def __call__(
        self,
        issue: Issue = None,
        *,
        type: IssueType,
        subject: Union["Symbol", "Statement", "File", None],
        **kwargs,
    ):
        issue = issue or Issue(type, subject, **kwargs)
        if self.on_issue:
            self.on_issue(issue)


raise_if_error = IssueRaiser(kinds=[IssueKind.ERROR])
raise_any = IssueRaiser(kinds=[IssueKind.ERROR, IssueKind.WARNING])
ignore_issues = IssueHandler()
