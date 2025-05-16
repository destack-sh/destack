from datetime import datetime
from functools import cached_property
from typing import TYPE_CHECKING, Optional, Union
from uuid import UUID

from bench.language.core import (
    IsClaimable,
    IsInstantiable,
    IsModal,
    IsOwnable,
    IsProcessable,
    IsTimed,
    IsTitled,
    NodeList,
    NodeType,
    PageNode,
    ProcessStatus,
    Subject,
    TextLineIn,
    TypeBase,
    p_node_children,
    p_node_parent,
    p_regular,
    text_line,
    timed_node_,
)
from bench.pb2 import TaskData

if TYPE_CHECKING:
    from bench.language import Node, NodeReference, Page

# pyright: reportIncompatibleVariableOverride=false


@timed_node_(NodeType.TASK)
class Task(
    IsTimed,
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
    parent: Union["Page", "Task", None] = p_node_parent(4, NodeType.PAGE, NodeType.TASK)
    # type?
    # priority?

    # routing
    due_at: Optional[datetime] = p_regular(50)
    assigned_to: Optional[Subject] = p_regular(51)
    nodes: list["Node"] = p_regular(52, description="The Nodes this Task is about.")
    if TYPE_CHECKING:
        assigned_to_ptr: Optional[NodeReference] = None
        assigned_to_id: Optional[UUID] = None
        assigned_to_type: Optional[NodeType] = None

    # ...IsProcessable[80-]

    tasks: NodeList["Task"] = p_node_children(NodeType.TASK)

    def start(self) -> None:
        self.status = ProcessStatus.RUNNING
        self.started_at = self.active_session._oracle.utc()

    def complete(self) -> None:
        self.status = ProcessStatus.COMPLETED
        self.terminated_at = self.active_session._oracle.utc()
        if self.started_at is not None:
            self.duration = self.terminated_at - self.started_at

    def reset(self) -> None:
        self.status = ProcessStatus.ASSIGNED if self.assigned_to_ptr else ProcessStatus.CREATED
        self.started_at = None
        self.terminated_at = None
        self.duration = None

    def fail(self) -> None:
        self.status = ProcessStatus.FAILED

    @cached_property
    def value_type(self) -> Optional["TypeBase"]:
        return None

    @staticmethod
    def new(
        title: "TextLineIn | None" = None,
        nodes: list["Node"] | None = None,
        **kwargs,
    ) -> "Task":
        task = Task(title=text_line(title) if title is not None else None, **kwargs)
        if nodes:
            task.nodes = nodes
        return task

    @staticmethod
    def manual(title: "TextLineIn | None" = None, nodes: list["Node"] | None = None) -> "Task":
        return Task.new(title=title, nodes=nodes)
