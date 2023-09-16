import abc
import uuid
from typing import TYPE_CHECKING, Optional, Union
from uuid import UUID

from bench.language import IssueType
from bench.language.const import MNT, IssueKind
from bench.language.module import node

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
    IssueType.TASK_IMPOSSIBLE.value: "task is impossible in current scope: {reason}",
    # notices
    IssueType.TASK_IS_STATIC.value: "task has no inputs and is not randomized",
    IssueType.TEXT_HAS_NO_EFFECT.value: "text has no effect here: {help}",
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
    IssueType.TASK_IMPOSSIBLE,
]
NOTICES = [IssueType.TASK_IS_STATIC, IssueType.TEXT_HAS_NO_EFFECT]
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


@node(mnt=MNT.Issue)
class Issue:
    id: Optional[UUID]
    ck: UUID
    kind: IssueKind
    type: IssueType
    message: str
    parent: Union["Statement", "File", None] = None

    def __init__(self, type: IssueType, parent: Union["Statement", "File", None], **kwargs):
        from bench.language import File, Statement
        from bench.language.const import NodePath, node_path_as_str

        if parent is not None and not isinstance(parent, (Statement, File)):
            raise ValueError(f"unexpected parent for issue {type}: {parent!r}")

        # auto convert kwargs
        for key, value in kwargs.items():
            if isinstance(value, (Statement, Statement, File)):
                kwargs[key] = value.name
            if isinstance(value, NodePath):
                kwargs[key] = node_path_as_str(value)

        self.type = type
        self.parent = parent
        if isinstance(parent, File):
            self.parent = parent
        elif isinstance(parent, (Statement, Statement)):
            self.parent = parent

        message = _ISSUE_MESSAGES.get(type.value)
        if "subject" in message:
            kwargs["subject"] = self.parent
        self.message = message.format(**kwargs)
        self.kind = _ISSUE_KIND_BY_TYPE[type]
        # generate id if not provided
        if "id" not in kwargs:
            self.id = uuid.uuid5(parent.id, type.value + self.message)
        else:
            self.id = kwargs.pop("id")
        self.ck = self.id

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
