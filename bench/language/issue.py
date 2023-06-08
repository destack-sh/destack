import enum
from dataclasses import dataclass
from typing import Optional

from bench.language.type import File, Statement, StatementPath, statement_path_as_str


class IssueKind(enum.StrEnum):
    ERROR = "error"
    WARNING = "warning"
    SUGGESTION = "suggestion"


class IssueType(enum.StrEnum):
    INTERNAL = "INTERNAL"
    UNKNOWN_IMPORT_SOURCE = "UNKNOWN_IMPORT_SOURCE"
    MISSING_REFERENCE = "MISSING_REFERENCE"
    UNDEFINED_REFERENCE = "UNDEFINED_REFERENCE"
    UNDEFINED_EXTERNAL_REFERENCE = "UNDEFINED_EXTERNAL_REFERENCE"
    EXTERNAL_LOOKUP_FAILED = "EXTERNAL_LOOKUP_FAILED"
    REFERENCE_TYPE_MISMATCH = "REFERENCE_TYPE_MISMATCH"
    AMBIGUOUS_DEFINITION = "AMBIGUOUS_DEFINITION"
    AMBIGUOUS_REQUIREMENT = "AMBIGUOUS_REQUIREMENT"
    CIRCULAR_ANCESTRY = "CIRCULAR_ANCESTRY"
    CIRCULAR_UNION = "CIRCULAR_UNION"
    MISMATCHED_UNION = "MISMATCHED_UNION"

    def __new__(cls, value, description):
        obj = object.__new__(cls)
        obj._value_ = value
        obj.description = description
        return obj

    @property
    def id(self) -> int:
        return self.value[0]


ISSUE_MESSAGES = {
    IssueType.INTERNAL: "Internal error",
    IssueType.UNKNOWN_IMPORT_SOURCE: "unknown import source {source}",
    IssueType.MISSING_REFERENCE: "missing reference",
    IssueType.UNDEFINED_REFERENCE: "undefined reference {path}",
    IssueType.UNDEFINED_EXTERNAL_REFERENCE: "undefined external reference {path} in module {module}",
    IssueType.EXTERNAL_LOOKUP_FAILED: "failed to lookup reference {path} in module {module}: {error}",
    IssueType.REFERENCE_TYPE_MISMATCH: "reference {resolved} is not of type {resolved}",
    IssueType.AMBIGUOUS_DEFINITION: "multiple definitions for {path}",
    IssueType.AMBIGUOUS_REQUIREMENT: "multiple requirements for {name}",
    IssueType.CIRCULAR_ANCESTRY: "circular ancestry via {path}",
    IssueType.CIRCULAR_UNION: "circular union via {path}",
    IssueType.MISMATCHED_UNION: "mismatched union at {node} vs {other} via {path}",
}


class Error(ValueError):
    def __init__(
        self,
        _t: IssueType,
        subject: File | Statement | None,
        cause: Optional[Exception] = None,
        **error_args,
    ):
        # convert error args as needed
        # StatementPath with statement_path_as_str
        error_args = {
            k1: statement_path_as_str(v1) if isinstance(v1, StatementPath) else v1
            for k1, v1 in error_args.items()
        }
        # noinspection StrFormat
        message1 = _t.description.format(**error_args)
        cause_context = f"\ncause: {cause.__class__.__name__} {cause}" if cause is not None else ""
        if isinstance(subject, Statement):
            thing_context = f" at {subject.infile_path}"
            self.scope = InterpScope.STATEMENT
        elif isinstance(subject, File):
            thing_context = f" in {subject.path}"
            self.scope = InterpScope.FILE
        else:
            thing_context = ""
            self.scope = InterpScope.MODULE
        result = message1, message1 + thing_context + cause_context
        self.short_message, message = result
        super().__init__(message)
        self.type = _t
        self.subject = subject
        self.related_statements = {k: v for k, v in error_args.items() if isinstance(v, Statement)}
        self.cause = cause

    def to_issue(self) -> "Issue":
        return Issue(
            kind=IssueKind.ERROR,
            type=self.type,
            scope=InterpScope.MODULE,
            message=self.short_message,
            verbose_message=self.args[0],
            file=self.subject if isinstance(self.subject, File) else None,
            statement=self.subject if isinstance(self.subject, Statement) else None,
        )


@dataclass
class Issue:
    kind: IssueKind
    type: IssueType
    scope: InterpScope
    message: str
    verbose_message: Optional[str] = None
    file: Optional[File] = None
    statement: Optional[Statement] = None
