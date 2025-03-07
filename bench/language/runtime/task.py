from datetime import datetime, timedelta
from functools import cached_property
from typing import TYPE_CHECKING, Any, Optional, Union, cast
from uuid import UUID

from bench.language.core import (
    BuiltinEnum,
    ColorType,
    CustomObject,
    EnumType,
    FieldType,
    IsInlinable,
    IsInstantiable,
    IsOwnable,
    IsRuntime,
    IsTimed,
    IsTraceable,
    NodeList,
    NodeType,
    PackageNode,
    StructType,
    Text,
    TypeBase,
    coerce_custom_object_scalar,
    enum_,
    p_internal,
    p_node_children,
    p_node_parent,
    p_regular,
    p_value_packed,
    p_value_runtime,
    timed_node_,
)
from bench.pb2 import TaskData

if TYPE_CHECKING:
    from bench.language import (
        Action,
        Class,
        Error,
        Flow,
        Interruption,
        NodeReference,
        Page,
        Plan,
        Run,
        Trigger,
    )

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.TASK_TYPE)
class TaskType(BuiltinEnum):
    GENERIC = 10, "Generic", "Describe a general purpose task", "far fa-square-check"
    SCHEDULED = 20, "Scheduled", "Schedule a task", "fas fa-calendar-days"
    RUN = 30, "Run", "Run a specific Node", "fas fa-play"
    INTERRUPTION = 40, "Interruption", "Handle an Interruption", "fas fa-hand"


@enum_(EnumType.TASK_STATUS)
class TaskStatus(BuiltinEnum):
    # pre
    CREATED = 1, None, None, "fas fa-clock", ColorType.GRAY
    ASSIGNED = 2, None, None, "far fa-rhombus", ColorType.GRAY
    # active
    RUNNING = 10, None, None, "fas fa-circle-notch", ColorType.GREEN
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
    IsInlinable,
    IsOwnable,
    IsRuntime,
    IsTraceable,
    IsInstantiable,
    PackageNode[TaskData],
):
    """A Task to accomplish something."""

    # meta
    parent: Union["Page", "Plan", "Task", "Run", None] = p_node_parent(
        4, NodeType.PAGE, NodeType.PLAN, NodeType.TASK, NodeType.RUN
    )
    type: TaskType = p_regular(30, default=TaskType.GENERIC)
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

    # status
    status: TaskStatus = p_regular(50, default=TaskStatus.CREATED)
    duration: Optional[timedelta] = p_internal(51, default=None)
    due_at: Optional[datetime] = p_regular(52, default=None)
    started_at: Optional[datetime] = p_internal(53, default=None)
    terminated_at: Optional[datetime] = p_internal(56, default=None)
    error: Optional["Error"] = p_internal(57, require=False, array=False, struct=StructType.ERROR)

    # content
    clazz: Optional["Class"] = p_internal(
        60,
        require=False,
        array=False,
        references=NodeType.CLASS,
        description="The Task class.",
    )
    target: Union["Flow", "Action", None] = p_regular(
        61,
        require=False,
        references=(NodeType.FLOW, NodeType.ACTION),
        description="The target Node to run.",
    )
    value_packed: Any = p_value_packed(65)
    value: Any = p_value_runtime(
        65, type=FieldType.INPUT, typ=lambda self: cast(Task, self).value_type
    )
    interruption: Optional["Interruption"] = p_regular(
        66,
        require=False,
        array=False,
        references=NodeType.INTERRUPTION,
        description="The Interruption this is about.",
        same_bench=True,
    )
    if TYPE_CHECKING:
        clazz_ptr: Optional[NodeReference] = None
        clazz_id: Optional[UUID] = None
        target_ptr: Optional[NodeReference] = None
        target_id: Optional[UUID] = None
        interruption_ptr: Optional[NodeReference] = None
        interruption_id: Optional[UUID] = None

    # flags
    is_manual: bool = p_regular(
        70, default=False, description="Whether to implement this Task manually."
    )

    triggers: NodeList["Trigger"] = p_node_children(NodeType.TRIGGER)
    tasks: NodeList["Task"] = p_node_children(NodeType.TASK)

    def complete(self, by: "Run") -> None:
        self.status = TaskStatus.COMPLETED

    def fail(self, by: "Run") -> None:
        self.error = by.error
        self.status = TaskStatus.FAILED

    @cached_property
    def value_type(self) -> Optional["TypeBase"]:
        node = self.target
        return (
            node.to_type_maybe(of="value", field_types=[FieldType.RESOURCE, FieldType.INPUT])
            if node is not None
            else None
        )

    @staticmethod
    def generic(
        name: str, text: "Text | None", clazz: "Class | None", is_manual: bool = False, **kwargs
    ) -> "Task":
        if clazz is not None:
            value_type = clazz.to_type_maybe(of="value")
            value = coerce_custom_object_scalar(kwargs, value_type)
        else:
            value = None
        task = Task(
            type=TaskType.GENERIC,
            name=name,
            text=text,
            clazz=clazz,
            value=value,
            is_manual=is_manual,
        )
        return task

    @staticmethod
    def run(
        name: str,
        node: "Flow | Action",
        value: CustomObject | None = None,
        *,
        text: "Text | None" = None,
        **kwargs,
    ) -> "Task":
        from bench.language import Action, Block

        if not isinstance(node, (Block, Action)):
            raise ValueError(f"invalid node type for Call: {type(node)}")

        value_type = node.to_type_maybe(
            of="value", field_types=[FieldType.RESOURCE, FieldType.INPUT]
        )
        assert value_type is not None, f"no call value type for {node!r}"
        value = coerce_custom_object_scalar(value or kwargs, value_type)
        task = Task(type=TaskType.RUN, name=name, target=node, text=text, value=value)
        return task
