import uuid
from typing import TYPE_CHECKING, Optional, Union
from uuid import UUID

from bench.language.node import Package
from bench.language.const import IssueKind, IssueType, NodeType, StructType, BenchError
from bench.language.expression import FieldPath
from bench.language.node import Node, node, node_parent, struct_property
from bench.language.validation import enum_validator

if TYPE_CHECKING:
    from bench.language.block import Block

# separate from enum so that it's a simple StrEnum
_ISSUE_MESSAGES = {
    # errors
    IssueType.INTERNAL.value: "internal error",
    IssueType.MISSING_REFERENCE.value: "missing reference at {path}",
    # warnings
    IssueType.AMBIGUOUS_DEFINITION.value: "multiple definitions for {path}",
    IssueType.TASK_MISSING_IO.value: "task has no inputs or outputs",
    # notices
    IssueType.BAD_NAME.value: "name does not follow conventions",
    IssueType.TASK_IS_STATIC.value: "task has no inputs and is not randomized",
}

ISSUE_TYPES = tuple(IssueType)

ERRORS: tuple[IssueType, ...] = tuple(t for t in ISSUE_TYPES if t.id < 100)
WARNINGS: tuple[IssueType, ...] = tuple(t for t in ISSUE_TYPES if 100 <= t.id < 200)
NOTICES: tuple[IssueType, ...] = tuple(t for t in ISSUE_TYPES if 200 <= t.id < 300)
_missing_issue_types = set(IssueType) - set(ERRORS) - set(WARNINGS) - set(NOTICES)
assert not _missing_issue_types, f"missing issue types: {_missing_issue_types}"
_ISSUE_KIND_BY_TYPE = {
    **{error: IssueKind.ERROR for error in ERRORS},
    **{warning: IssueKind.WARNING for warning in WARNINGS},
    **{notice: IssueKind.NOTICE for notice in NOTICES},
}


class IssueError(BenchError, ValueError):
    def __init__(self, issue: "Issue", cause: Exception | None = None):
        super().__init__(f"{issue!r}: {issue.message}")
        self.issue = issue
        self.cause = cause


@node(NodeType.ISSUE)
class Issue(Node):
    parent: Union["Block", "Package"] = node_parent(4, NodeType.BLOCK, NodeType.PACKAGE)
    type: IssueType = struct_property(30, validate=enum_validator(IssueType))
    kind: IssueKind = struct_property(31, default=None, validate=enum_validator(IssueKind))
    message: str = struct_property(32, default=None)
    path: Optional[FieldPath] = struct_property(
        34, default=None, require=False, array=False, struct=StructType.FIELD_PATH
    )
    properties: Optional[list[int]] = struct_property(35, default=None)

    def _init_inner(self):
        # make message
        message = _ISSUE_MESSAGES.get(self.type.value) or self.type.value
        kwargs = {}
        if "subject" in message:
            kwargs["subject"] = self.subject or self.parent
        if "path" in message:
            kwargs["path"] = self.path
        if "other" in message:
            kwargs["other"] = self.other
        self.message = message.format(**kwargs)
        self.kind = _ISSUE_KIND_BY_TYPE[self.type]

    def __content_str__(self):
        return f"{self.kind}: {self.type} {self.message}"

    @property
    def subject_id(self) -> UUID | None:
        return self.parent.id if self.parent is not None else None

    @staticmethod
    def from_subject(
        subject: Node,
        type: IssueType,
        message: str = None,
        path: Optional[FieldPath] = None,
        properties: Optional[list[str]] = None,
    ) -> "Issue":
        assert isinstance(subject, Node), f"invalid subject: {subject!r}"
        issue_ck = uuid.uuid5(subject.id, (type.value + path + properties))
        issue_id = issue_ck  # not sure?
        if subject.metatype not in (NodeType.BLOCK, NodeType.PACKAGE):
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
