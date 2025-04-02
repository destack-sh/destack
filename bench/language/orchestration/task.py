from datetime import datetime
from functools import cached_property
from typing import TYPE_CHECKING, Any, Optional, Union, cast
from uuid import UUID

from bench.language.core import (
    BuiltinEnum,
    ColorType,
    CustomObject,
    EnumType,
    FieldType,
    InlineNode,
    IsClaimable,
    IsInstantiable,
    IsModal,
    IsOwnable,
    IsRuntimeControllable,
    IsTimed,
    IsTitled,
    IsType,
    NodeList,
    NodeType,
    StructType,
    Text,
    TextLineIn,
    coerce_custom_object_scalar,
    enum_,
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


@enum_(EnumType.TASK_TYPE)
class TaskType(BuiltinEnum):
    GENERAL = 10, "General", "Describe a general purpose task", "far fa-square-check"
    RUN = 30, "Run", "Run a specific Node", "fas fa-play"


@enum_(EnumType.TASK_STATUS)
class TaskStatus(BuiltinEnum):
    # pre
    CREATED = 1, None, None, "fas fa-clock", ColorType.GRAY
    ASSIGNED = 2, None, None, "far fa-rhombus", ColorType.GRAY
    # active
    RUNNING = 10, None, None, "fas fa-circle-notch", ColorType.BLUE
    # waiting
    WAITING = 22, None, None, "fas fa-circle-pause", ColorType.PINK
    REVIEWING = 23, None, None, "fas fa-circle-pause", ColorType.PINK
    # terminal
    CANCELLED = 30, None, None, "fas fa-circle-xmark", ColorType.RED
    ABORTED = 31, None, None, "fas fa-skull", ColorType.RED
    FAILED = 32, None, None, "fas fa-circle-exclamation", ColorType.RED
    COMPLETED = 33, None, None, "fas fa-circle-check", ColorType.GREEN


@timed_node_(NodeType.TASK)
class Task(
    IsTimed,
    IsOwnable,
    IsClaimable,
    IsRuntimeControllable,
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
    type: TaskType = p_regular(30, default=TaskType.GENERAL)
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

    # status [80-90]
    status: TaskStatus = p_regular(80, default=TaskStatus.CREATED)

    triggers: NodeList["Trigger"] = p_node_children(NodeType.TRIGGER)
    tasks: NodeList["Task"] = p_node_children(NodeType.TASK)

    def start(self) -> None:
        self.status = TaskStatus.RUNNING
        self.started_at = self.active_session._oracle.utc()

    def complete(self) -> None:
        self.status = TaskStatus.COMPLETED
        self.terminated_at = self.active_session._oracle.utc()
        if self.started_at is not None:
            self.duration = self.terminated_at - self.started_at

    def stop(self) -> None:
        self.stopped_at = self.active_session._oracle.utc()

    def fail(self, error: "Error | None" = None) -> None:
        self.error = error
        self.status = TaskStatus.FAILED

    @cached_property
    def value_type(self) -> Optional["IsType"]:
        if (tool := self.tool) is not None:
            return tool.to_type_maybe(of="value", field_types=[FieldType.INPUT])
        elif (node := self.target) is not None:
            return node.to_type_maybe(of="value", field_types=[FieldType.INPUT])
        else:
            return None

    @staticmethod
    def general(
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
            type=TaskType.GENERAL,
            title=text_line(title) if title is not None else None,
            text=text,
            clazz=clazz,
            value=value,
            is_manual=is_manual,
        )
        return task

    @staticmethod
    def run(
        title: "TextLineIn | None",
        node: "Flow | Action",
        value: CustomObject | None = None,
        *,
        tool: "Flow | Action | None" = None,
        text: "Text | None" = None,
        **kwargs,
    ) -> "Task":
        from bench.language import Action, Block

        if not isinstance(node, (Block, Action)):
            raise ValueError(f"invalid node type for Call: {type(node)}")

        if tool is not None:
            value_type = tool.to_type_maybe(of="value", field_types=[FieldType.INPUT])
            assert value_type is not None, f"no call value type for tool {tool!r}"
        else:
            value_type = node.to_type_maybe(of="value", field_types=[FieldType.INPUT])
            assert value_type is not None, f"no call value type for node {node!r}"
        value = coerce_custom_object_scalar(value or kwargs, value_type)
        task = Task(
            type=TaskType.RUN,
            title=text_line(title) if title is not None else None,
            target=node,
            value=value,
            text=text,
        )
        return task
