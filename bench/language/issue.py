import abc
from typing import TYPE_CHECKING, Optional, Union
from uuid import UUID

from bench.language import IssueType
from bench.language.const import IssueKind, NodeType
from bench.language.module import Node, bproperty, node, nparent
from bench.language.validation import enum_validator

if TYPE_CHECKING:
    from bench.language.file import File
    from bench.language.statement import Statement

# separate from enum so that it's a simple StrEnum
_ISSUE_MESSAGES = {
    # errors
    IssueType.INTERNAL.value: "internal error",
    IssueType.MISSING_REFERENCE.value: "missing reference at {path}",
    IssueType.CIRCULAR_ANCESTRY.value: "circular ancestry via {path}",
    IssueType.CIRCULAR_UNION.value: "circular union via {path}",
    IssueType.MISMATCHED_UNION.value: "mismatched union at {subject} vs {other}",
    # warnings
    IssueType.CODE_NOT_EXPORTABLE.value: "code {subject} is not exportable",
    IssueType.CODE_REFERENCE_NOT_EXPORTED.value: "code {path} is not exported",
    IssueType.AMBIGUOUS_DEFINITION.value: "multiple definitions for {path}",
    IssueType.TASK_MISSING_IO.value: "task has no inputs or outputs",
    # notices
    IssueType.TASK_IS_STATIC.value: "task has no inputs and is not randomized",
}

ERRORS = [
    IssueType.INTERNAL,
    IssueType.UNKNOWN_IMPORT_SOURCE,
    IssueType.MISSING_REFERENCE,
    IssueType.CIRCULAR_ANCESTRY,
    IssueType.CIRCULAR_UNION,
    IssueType.MISMATCHED_UNION,
]
WARNINGS = [
    IssueType.AMBIGUOUS_DEFINITION,
    IssueType.CODE_NOT_EXPORTABLE,
    IssueType.CODE_REFERENCE_NOT_EXPORTED,
    IssueType.CODE_NOT_CACHEABLE,
    IssueType.TASK_MISSING_IO,
]
NOTICES = [IssueType.TASK_IS_STATIC]
_missing_issue_types = set(IssueType) - set(ERRORS) - set(WARNINGS) - set(NOTICES)
assert not _missing_issue_types, f"missing issue types: {_missing_issue_types}"
_ISSUE_KIND_BY_TYPE = {
    **{error: IssueKind.Error for error in ERRORS},
    **{warning: IssueKind.Warning for warning in WARNINGS},
    **{notice: IssueKind.Notice for notice in NOTICES},
}


class BenchError(ValueError):
    def __init__(self, issue: "Issue", **kwargs):
        super().__init__(issue.message.format(**kwargs))
        self.issue = issue


@node(node_type=NodeType.ISSUE)
class Issue(Node):
    parent: Union["Statement", "File", None] = nparent(NodeType.STATEMENT, NodeType.FILE)
    type: IssueType = bproperty(is_required=True, validate=enum_validator(IssueType))
    kind: IssueKind = bproperty(default=None, validate=enum_validator(IssueKind))
    message: str = bproperty(default=None)
    subject: Optional[Node] = bproperty(default=None)
    path: Optional[str] = bproperty(default=None)
    other: Optional[Node] = bproperty(default=None)

    def _init_inner(self):
        # make message
        message = _ISSUE_MESSAGES[self.type.value]
        kwargs = {}
        if "subject" in message:
            kwargs["subject"] = self.subject or self.parent
        if "path" in message:
            kwargs["path"] = self.path
        if "other" in message:
            kwargs["other"] = self.other
        self.message = message.format(**kwargs)
        self.kind = _ISSUE_KIND_BY_TYPE[self.type]

    def __str__(self):
        return f"{self.parent} {self.kind}: {self.type} {self.message}"

    def __repr__(self):
        return f"<Issue {self}>"

    @property
    def parent_id(self) -> UUID | None:
        return self.parent.id if self.parent is not None else None

    @property
    def subject_id(self) -> UUID | None:
        return self.parent.id if self.parent is not None else None

    def to_error(self) -> BenchError:
        return BenchError(self)


class IssueHandler(abc.ABC):
    def __call__(
        self,
        issue: "Issue" = None,
        *,
        subject: Union["Statement", "File", None],
        type: IssueType,
        **kwargs,
    ):
        pass
