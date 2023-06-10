import enum
from dataclasses import dataclass
from typing import TYPE_CHECKING, Optional, Union

from bench.language.const import InterpScope, StatementPath, statement_path_as_str

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
        self.issues.append(issue)


raise_if_error = IssueRaiser(kinds=[IssueKind.ERROR])
raise_any = IssueRaiser(kinds=[IssueKind.ERROR, IssueKind.WARNING])
ignore_issues = IssueHandler()
