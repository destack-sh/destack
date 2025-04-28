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
    IsType,
    NodeList,
    NodeType,
    PageNode,
    ProcessStatus,
    TextLineIn,
    p_node_children,
    p_node_parent,
    p_regular,
    text_line,
    timed_node_,
)
from bench.pb2 import TaskData

if TYPE_CHECKING:
    from bench.language import (
        Error,
        Node,
        NodeReference,
        Page,
        Plan,
        Run,
        Trigger,
    )

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
    """A Task to accomplish something."""

    # meta
    parent: Union["Page", "Plan", "Task", "Run", None] = p_node_parent(
        4, NodeType.PAGE, NodeType.PLAN, NodeType.TASK, NodeType.RUN
    )
    # type?
    # priority?

    # routing
    due_at: Optional[datetime] = p_regular(50, default=None)
    nodes: list["Node"] = p_regular(
        52, require=False, array=True, references="any", description="The Nodes this Task is about."
    )
    if TYPE_CHECKING:
        nodes_ptr: Optional[NodeReference] = None
        nodes_id: Optional[UUID] = None

    # ...IsProcessable[80-]

    triggers: NodeList["Trigger"] = p_node_children(NodeType.TRIGGER)
    tasks: NodeList["Task"] = p_node_children(NodeType.TASK)

    def start(self) -> None:
        self.status = ProcessStatus.RUNNING
        self.started_at = self.active_session._oracle.utc()

    def complete(self) -> None:
        self.status = ProcessStatus.COMPLETED
        self.terminated_at = self.active_session._oracle.utc()
        if self.started_at is not None:
            self.duration = self.terminated_at - self.started_at

    def stop(self) -> None:
        self.requested_stop_at = self.active_session._oracle.utc()

    def fail(self, error: "Error | None" = None) -> None:
        self.error = error
        self.status = ProcessStatus.FAILED

    @cached_property
    def value_type(self) -> Optional["IsType"]:
        return None

    @staticmethod
    def new(
        title: "TextLineIn | None" = None,
        nodes: list["Node"] | None = None,
    ) -> "Task":
        task = Task(title=text_line(title) if title is not None else None)
        if nodes:
            task.nodes = nodes
        return task

    @staticmethod
    def manual(title: "TextLineIn | None" = None, nodes: list["Node"] | None = None) -> "Task":
        return Task.new(title=title, nodes=nodes)
