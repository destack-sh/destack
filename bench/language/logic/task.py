from datetime import datetime
from typing import TYPE_CHECKING, Optional, Union

from fastuuid import UUID

from bench.language.core import (
    IsClaimable,
    IsInstantiable,
    IsModal,
    IsOwnable,
    IsProcessable,
    IsTitled,
    NodeType,
    PageNode,
    ProcessStatus,
    Subject,
    TextLineIn,
    node_,
    p_node_parent,
    p_regular,
    text_line,
)
from bench.pb2 import TaskData

if TYPE_CHECKING:
    from bench.language import NodeReference, Page

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.TASK)
class Task(
    IsOwnable,
    IsClaimable,
    IsProcessable,
    IsModal,
    IsTitled,
    IsInstantiable,
    PageNode[TaskData],
):
    """A Task is like a to do item."""

    # meta
    parent: Union["Page", "Task", None] = p_node_parent(4)
    # type?
    # priority?

    # routing
    due_at: Optional[datetime] = p_regular(50)
    assigned_to: Optional[Subject] = p_regular(51)
    if TYPE_CHECKING:
        assigned_to_ptr: Optional[NodeReference] = None
        assigned_to_id: Optional[UUID] = None
        assigned_to_type: Optional[NodeType] = None

    # ...IsProcessable[80-]

    def start(self) -> None:
        self.status = ProcessStatus.RUNNING
        self.started_at = self.active_session.oracle.utc()

    def complete(self) -> None:
        self.status = ProcessStatus.COMPLETED
        self.terminated_at = self.active_session.oracle.utc()
        if self.started_at is not None:
            self.duration = self.terminated_at - self.started_at

    def reset(self) -> None:
        self.status = ProcessStatus.ASSIGNED if self.assigned_to_ptr else ProcessStatus.CREATED
        self.started_at = None
        self.terminated_at = None
        self.duration = None

    def fail(self) -> None:
        self.status = ProcessStatus.FAILED

    @staticmethod
    def new(
        title: "TextLineIn | None" = None,
        **kwargs,
    ) -> "Task":
        task = Task(title=text_line(title) if title is not None else None, **kwargs)
        return task
