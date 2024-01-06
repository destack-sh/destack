import uuid
from typing import TYPE_CHECKING, Optional, Union
from uuid import UUID

from bench.language.const import IssueKind, IssueType, NodeType
from bench.language.module import Node, node, node_parent, struct_property
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
    IssueType.INVALID_DATA,
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
    **{error: IssueKind.ERROR for error in ERRORS},
    **{warning: IssueKind.WARNING for warning in WARNINGS},
    **{notice: IssueKind.NOTICE for notice in NOTICES},
}


class BenchError(ValueError):
    def __init__(self, issue: "Issue", **kwargs):
        super().__init__(issue.message.format(**kwargs))
        self.issue = issue


@node(NodeType.ISSUE)
class Issue(Node):
    parent: Union["Statement", "File"] = node_parent(4, NodeType.STATEMENT, NodeType.FILE)
    type: IssueType = struct_property(30, require=True, validate=enum_validator(IssueType))
    kind: IssueKind = struct_property(31, default=None, validate=enum_validator(IssueKind))
    message: str = struct_property(32, default=None)
    path: Optional[str] = struct_property(34, default=None)
    properties: list[str] = struct_property(35, default=None)

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
    def subject_id(self) -> UUID | None:
        return self.parent.id if self.parent is not None else None

    def to_error(self) -> BenchError:
        return BenchError(self)

    @staticmethod
    def from_subject(
        subject: Node,
        type: IssueType,
        message: str = None,
        path: Optional[str] = None,
        properties: Optional[list[str]] = None,
    ) -> "Issue":
        from bench.language import File, Statement

        assert isinstance(subject, Node), f"invalid subject: {subject!r}"
        issue_ck = uuid.uuid5(subject.id, (type.value + (message or "")))
        issue_id = issue_ck  # not sure?
        if not isinstance(subject, (Statement, File)):
            subject = subject.parent  # fields don't have issues (yet)
        issue = Issue(
            id=issue_id,
            ck=issue_ck,
            type=type,
            subject=subject,
            parent=None,
            path=path,
            properties=properties,
            message=message,
        )
        # don't set parent yet because it would append it to the issues list
        return issue


class IssueHandler:
    def __call__(
        self,
        subject: "Node",
        type: IssueType,
        message: Optional[str] = None,
        path: Optional[str] = None,
        properties: list[str] | None = None,
        **kwargs,
    ):
        pass
