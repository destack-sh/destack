from datetime import datetime
from functools import cached_property
from typing import TYPE_CHECKING, Any, Optional, Union, cast
from uuid import UUID

from bench.language.core import (
    FieldType,
    InlineNode,
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
    ProcessStatus,
    StructType,
    Text,
    TextLineIn,
    coerce_custom_object_scalar,
    p_internal,
    p_node_children,
    p_node_parent,
    p_regular,
    p_value_packed,
    p_value_runtime,
    text_line,
    timed_node_,
)
from bench.pb2 import TaskData

if TYPE_CHECKING:
    from bench.language import (
        Action,
        Class,
        Error,
        Flow,
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
    InlineNode[TaskData],
):
    """A Task to accomplish something."""

    # meta
    parent: Union["Page", "Plan", "Task", "Run", None] = p_node_parent(
        4, NodeType.PAGE, NodeType.PLAN, NodeType.TASK, NodeType.RUN
    )
    # priority?
    implemented_by: Optional["Run"] = p_internal(
        41,
        require=False,
        array=False,
        same_bench=True,
        references=NodeType.RUN,
        description="The Run that implements this Task.",
    )
    if TYPE_CHECKING:
        implemented_by_ptr: Optional[NodeReference] = None
        implemented_by_id: Optional[UUID] = None

    # routing
    due_at: Optional[datetime] = p_regular(52, default=None)
    text: Optional["Text"] = p_regular(
        60, default=None, require=False, array=False, struct=StructType.TEXT
    )
    target: Union["Flow", "Action", None] = p_regular(
        62,
        require=False,
        references=(NodeType.FLOW, NodeType.ACTION),
        description="The target Node at which to run this Task.",
    )
    tool: Union["Flow", "Action", None] = p_regular(
        63,
        require=False,
        references=(NodeType.FLOW, NodeType.ACTION),
        description="The tool Node to use (at the target).",
    )
    is_manual: bool = p_regular(
        69, default=False, description="Whether to implement this Task manually."
    )
    if TYPE_CHECKING:
        target_ptr: Optional[NodeReference] = None
        target_id: Optional[UUID] = None
        tool_ptr: Optional[NodeReference] = None
        tool_id: Optional[UUID] = None

    # content
    clazz: Optional["Class"] = p_internal(
        70,
        require=False,
        array=False,
        references=NodeType.CLASS,
        description="The Task class.",
    )
    value_packed: Any = p_value_packed(71)
    value: Any = p_value_runtime(
        71, type=FieldType.INPUT, typ=lambda self: cast(Task, self).value_type
    )
    nodes: list["Node"] = p_regular(
        72, require=False, array=True, references="any", description="The Nodes this Task is about."
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
        if (tool := self.tool) is not None:
            return tool.to_type_maybe(of="value", field_types=[FieldType.INPUT])
        elif (node := self.target) is not None:
            return node.to_type_maybe(of="value", field_types=[FieldType.INPUT])
        else:
            return None

    @staticmethod
    def new(
        title: "TextLineIn | None",
        text: "Text | None",
        clazz: "Class | None",
        is_manual: bool = False,
        **kwargs,
    ) -> "Task":
        if clazz is not None:
            value_type = clazz.to_type_maybe(of="value")
            value = coerce_custom_object_scalar(kwargs, value_type)
        else:
            value = None
        task = Task(
            title=text_line(title) if title is not None else None,
            text=text,
            clazz=clazz,
            value=value,
            is_manual=is_manual,
        )
        return task
