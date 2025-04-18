from datetime import datetime
from functools import cached_property
from typing import TYPE_CHECKING, Optional, Union
from uuid import UUID

from bench.language.core import (
    BuiltinEnum,
    EnumType,
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
    StructType,
    Text,
    TextLineIn,
    enum_,
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


@enum_(EnumType.TASK_TYPE)
class TaskType(BuiltinEnum):
    """The type of Task."""

    MANUAL = 10, "Manual", "Manual Task", "fas fa-pencil"


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
    # priority?
    type: TaskType = p_regular(30, require=True)

    # routing
    due_at: Optional[datetime] = p_regular(50, default=None)
    text: Optional["Text"] = p_regular(
        51, default=None, require=False, array=False, struct=StructType.TEXT
    )
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
        type: TaskType = TaskType.MANUAL,
        title: "TextLineIn | None" = None,
        text: "Text | None" = None,
    ) -> "Task":
        task = Task(
            title=text_line(title) if title is not None else None,
            text=text,
            type=type,
        )
        return task

    @staticmethod
    def manual(title: "TextLineIn | None" = None, text: "Text | None" = None) -> "Task":
        return Task.new(type=TaskType.MANUAL, title=title, text=text)
