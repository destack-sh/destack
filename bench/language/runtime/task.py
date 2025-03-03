from datetime import datetime, timedelta
from functools import cached_property
from typing import TYPE_CHECKING, Any, Optional, Union, cast
from uuid import UUID

from bench.language.core import (
    OWNER_TYPES,
    BuiltinEnum,
    ColorType,
    CustomObject,
    EnumType,
    FieldType,
    HasRuntimeContext,
    HasTimeIdentity,
    HasTrace,
    IsInlinable,
    NodeList,
    NodeType,
    Owner,
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
        NodeReference,
        Package,
        Page,
        Plan,
        Run,
        Trigger,
    )

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.TASK_TYPE)
class TaskType(BuiltinEnum):
    AUTOMATIC = 10, "Automatic", "Implement automatically"
    STATE = 20, "State", "Track progress inside a Run"
    RUN = 30, "Run", "Run a specific Runnable"


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
class Task(HasTimeIdentity, IsInlinable, PackageNode[TaskData], HasRuntimeContext, HasTrace):
    """A Task to accomplish something."""

    # meta
    parent: Union["Package", "Page", "Plan", "Task", "Run", None] = p_node_parent(
        4, NodeType.PACKAGE, NodeType.PAGE, NodeType.PLAN, NodeType.TASK, NodeType.RUN
    )
    type: TaskType = p_regular(30)
    # priority?

    # status
    status: TaskStatus = p_regular(50, default=TaskStatus.CREATED)
    duration: Optional[timedelta] = p_internal(51, default=None)
    due_at: Optional[datetime] = p_regular(52, default=None)
    started_at: Optional[datetime] = p_internal(53, default=None)
    terminated_at: Optional[datetime] = p_internal(56, default=None)
    error: Optional["Error"] = p_internal(57, require=False, array=False, struct=StructType.ERROR)
    owned_by: Optional[Owner] = p_internal(58, require=False, array=False, references=OWNER_TYPES)
    if TYPE_CHECKING:
        owned_by_ptr: Optional[NodeReference] = None
        owned_by_id: Optional[UUID] = None

    # content
    node: Union["Flow", "Action", None] = p_regular(
        60, require=True, references=(NodeType.FLOW, NodeType.ACTION)
    )
    clazz: Optional["Class"] = p_internal(
        61,
        require=False,
        array=False,
        references=NodeType.CLASS,
        description="The Task class.",
    )
    value_packed: Any = p_value_packed(62)
    value: Any = p_value_runtime(
        62, type=FieldType.INPUT, typ=lambda self: cast(Task, self).value_type
    )
    if TYPE_CHECKING:
        node_ptr: NodeReference | None = None
        node_id: str | None = None
        clazz_ptr: Optional[NodeReference] = None
        clazz_id: Optional[UUID] = None

    triggers: NodeList["Trigger"] = p_node_children(NodeType.TRIGGER)
    tasks: NodeList["Task"] = p_node_children(NodeType.TASK)

    def complete(self, by: "Run") -> None:
        self.status = TaskStatus.COMPLETED

    def fail(self, by: "Run") -> None:
        self.error = by.error
        self.status = TaskStatus.FAILED

    @cached_property
    def value_type(self) -> Optional["TypeBase"]:
        node = self.node
        return (
            node.to_type_maybe(of="value", field_types=[FieldType.VARIABLE, FieldType.INPUT])
            if node is not None
            else None
        )

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
            of="value", field_types=[FieldType.VARIABLE, FieldType.INPUT]
        )
        assert value_type is not None, f"no call value type for {node!r}"
        value = coerce_custom_object_scalar(value or kwargs, value_type)
        task = Task(type=TaskType.RUN, name=name, node=node, text=text, value=value)
        return task
