from typing import TYPE_CHECKING, Optional, Union
from uuid import UUID

from bench.language.const import BenchError, EnumType, IssueKind, NodeType, StructType, enum_
from bench.language.node import (
    LINK_TARGET_NODE_TYPES,
    Node,
    PackageNode,
    Property,
    local_node,
)
from bench.language.property import p_node_parent, p_regular
from bench.language.text import Text
from bench.language.validation import TITLE_CONSTRAINT
from bench.proto.wire import IssueData
from bench.utils.func import IdEnum

if TYPE_CHECKING:
    from bench.language import Block, Field, Path, Step, View

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.ISSUE_TYPE)
class IssueType(IdEnum):
    """Built-in issue types."""

    # errors
    MISSING_REFERENCE = 1
    CIRCULAR_BASE = 2
    MISMATCHED_BASE = 3

    # warnings
    AMBIGUOUS_NAME = 100

    # information
    ...

    # hints
    ...

    @property
    def kind(self) -> IssueKind:
        if self.id < 100:
            return IssueKind.ERROR
        elif self.id < 200:
            return IssueKind.WARNING
        elif self.id < 300:
            return IssueKind.INFO
        else:
            return IssueKind.HINT


NOTICE_TYPES = tuple(IssueType)


class IssueError(BenchError, ValueError):
    def __init__(self, notice: "Issue", cause: Exception | None = None):
        super().__init__(repr(notice))
        self.notice = notice
        self.cause = cause


IssueParent = Union["Block", "Field", "Step", "View"]
ISSUE_PARENT_TYPES: tuple[NodeType, ...] = (
    NodeType.BLOCK,
    NodeType.FIELD,
    NodeType.STEP,
    NodeType.VIEW,
)


@local_node(NodeType.ISSUE)
class Issue(PackageNode[IssueData]):
    """
    A diagnostic regarding something in the Bench source.
    """

    parent: IssueParent | None = p_node_parent(4, *ISSUE_PARENT_TYPES)
    kind: IssueKind = p_regular(30, default=None)
    type: IssueType = p_regular(31)
    subject: Node = p_regular(33, require=False, references=LINK_TARGET_NODE_TYPES)
    path: Optional["Path"] = p_regular(34, require=False, array=False, struct=StructType.PATH)
    properties: Optional[list[Property]] = p_regular(
        35, require=False, array=True, struct=StructType.PROPERTY_REFERENCE
    )

    # content
    title: Optional[str] = p_regular(40, require=False, default=None, constraint=TITLE_CONSTRAINT)
    text: Optional["Text"] = p_regular(41, require=False, default=None, struct=StructType.TEXT)
    # value_packed, value: ... # custom value

    def __content_str__(self):
        return f"{self.kind.bench_name}: {self.type} {self.text}"

    @property
    def subject_id(self) -> UUID | None:
        return self.parent.id if self.parent is not None else None
