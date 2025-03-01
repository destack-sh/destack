from datetime import datetime, timedelta
from typing import TYPE_CHECKING, Optional, Union
from uuid import UUID

from bench.language.core import (
    OWNER_TYPES,
    BuiltinEnum,
    ColorType,
    EnumType,
    HasTimeIdentity,
    InlineSourceNode,
    NodeList,
    NodeType,
    Owner,
    StructType,
    enum_,
    p_internal,
    p_node_children,
    p_node_parent,
    p_regular,
    timed_node_,
)
from bench.pb2 import PlanData

if TYPE_CHECKING:
    from bench.language import (
        CallExecutionMode,
        CallFailureMode,
        CallTerminationMode,
        Error,
        NodeReference,
        Package,
        Page,
        Run,
        Task,
    )

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.PLAN_TYPE)
class PlanType(BuiltinEnum):
    CALL = 1, "Run"


@enum_(EnumType.PLAN_STATUS)
class PlanStatus(BuiltinEnum):
    # pre
    CREATED = 1, None, None, "fas fa-clock", ColorType.GRAY
    # active
    RUNNING = 10, None, None, "fas fa-circle-notch", ColorType.GREEN
    # terminal
    CANCELLED = 30, None, None, "fas fa-circle-xmark", ColorType.RED
    ABORTED = 31, None, None, "fas fa-skull", ColorType.RED
    FAILED = 32, None, None, "fas fa-circle-exclamation", ColorType.RED
    COMPLETED = 33, None, None, "fas fa-circle-check", ColorType.GREEN


@timed_node_(NodeType.PLAN)
class Plan(HasTimeIdentity, InlineSourceNode[PlanData]):
    """A Plan for a sequence of Runs or something."""

    # meta
    parent: Union["Package", "Page", "Run", None] = p_node_parent(
        4, NodeType.PACKAGE, NodeType.PAGE, NodeType.RUN
    )
    type: PlanType = p_regular(30)
    execution: "CallExecutionMode" = p_internal(40)
    on_terminate: "CallTerminationMode" = p_internal(41)
    on_error: "CallFailureMode" = p_internal(42)

    # status
    status: PlanStatus = p_internal(50, default=PlanStatus.CREATED)
    duration: Optional[timedelta] = p_internal(51, default=None)
    started_at: Optional[datetime] = p_internal(52, default=None)
    terminated_at: Optional[datetime] = p_internal(55, default=None)
    error: Optional["Error"] = p_internal(56, require=False, array=False, struct=StructType.ERROR)
    owned_by: Optional[Owner] = p_internal(57, require=False, array=False, references=OWNER_TYPES)
    if TYPE_CHECKING:
        owned_by_ptr: Optional[NodeReference] = None
        owned_by_id: Optional[UUID] = None

    tasks: NodeList["Task"] = p_node_children(NodeType.TASK)

    def complete(self, by: "Run") -> None:
        self.status = PlanStatus.COMPLETED

    def fail(self, by: "Run") -> None:
        self.error = by.error
        self.status = PlanStatus.FAILED
