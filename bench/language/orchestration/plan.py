from datetime import datetime, timedelta
from typing import TYPE_CHECKING, Optional, Union
from uuid import UUID

from bench.language.core import (
    BuiltinEnum,
    ColorType,
    EnumType,
    InlineNode,
    IsInstantiable,
    IsModal,
    IsOwnable,
    IsRuntime,
    IsTimed,
    IsTitled,
    LocalNodeList,
    NodeType,
    StructType,
    TextLineIn,
    enum_,
    p_internal,
    p_node_children,
    p_node_parent,
    p_regular,
    text_line,
    timed_node_,
)
from bench.pb2 import PlanData

if TYPE_CHECKING:
    from bench.language import Error, NodeReference, Page, Run, Task, Trigger

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.PLAN_TYPE)
class PlanType(BuiltinEnum):
    SERIAL = 10, "Serial"
    PARALLEL = 20, "Parallel"
    # QUEUE = 30, "Queue"


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

    @property
    def is_active(self) -> bool:
        return self >= 10 and self < 20

    @property
    def is_terminal(self) -> bool:
        return self >= 30


@enum_(EnumType.PLAN_TERMINATION_MODE)
class PlanTerminationMode(BuiltinEnum):
    PASS = 10, "Complete", "Complete the Plan"
    RETURN = 20, "Return", "Return to caller (for more planning)"


@enum_(EnumType.PLAN_FAILURE_MODE)
class PlanFailureMode(BuiltinEnum):
    FAIL = 10, "Fail", "Fail the Plan"
    END = 20, "Complete", "Complete the Plan"
    CONTINUE = 30, "Continue", "Continue the Plan (skip failures)"


@timed_node_(NodeType.PLAN)
class Plan(
    IsTimed,
    IsOwnable,
    IsRuntime,
    IsModal,
    IsInstantiable,
    IsTitled,
    InlineNode[PlanData],
):
    """A Plan for something expressed as a sequence of Tasks."""

    # meta
    parent: Union["Page", "Plan", "Run", None] = p_node_parent(
        4, NodeType.PAGE, NodeType.PLAN, NodeType.RUN
    )
    type: PlanType = p_regular(30)
    on_terminate: "PlanTerminationMode" = p_internal(41)
    on_failure: "PlanFailureMode" = p_internal(42)
    implemented_by: Optional["Run"] = p_internal(
        43, require=False, array=False, references=NodeType.RUN, same_bench=True
    )
    if TYPE_CHECKING:
        implemented_by_ptr: Optional[NodeReference] = None
        implemented_by_id: Optional[UUID] = None

    # status
    status: PlanStatus = p_internal(50, default=PlanStatus.CREATED)
    duration: Optional[timedelta] = p_internal(51, default=None)
    started_at: Optional[datetime] = p_internal(52, default=None)
    terminated_at: Optional[datetime] = p_internal(55, default=None)
    error: Optional["Error"] = p_internal(56, require=False, array=False, struct=StructType.ERROR)

    triggers: LocalNodeList["Trigger"] = p_node_children(NodeType.TRIGGER)
    plans: LocalNodeList["Plan"] = p_node_children(NodeType.PLAN)
    tasks: LocalNodeList["Task"] = p_node_children(NodeType.TASK)

    def complete(self) -> None:
        self.status = PlanStatus.COMPLETED

    def fail(self, error: "Error | None") -> None:
        self.error = error
        self.status = PlanStatus.FAILED

    @staticmethod
    def serial(
        title: "TextLineIn",
        *tasks: "Task",
        on_terminate: PlanTerminationMode,
        on_failure: PlanFailureMode = PlanFailureMode.END,
    ) -> "Plan":
        plan = Plan(
            type=PlanType.SERIAL,
            title=text_line(title),
            on_terminate=on_terminate,
            on_failure=on_failure,
        )
        plan.tasks.extend(*tasks)
        return plan

    @staticmethod
    def parallel(
        title: "TextLineIn",
        *tasks: "Task",
        on_terminate: PlanTerminationMode,
        on_failure: PlanFailureMode = PlanFailureMode.END,
    ) -> "Plan":
        plan = Plan(
            type=PlanType.PARALLEL,
            title=text_line(title),
            on_terminate=on_terminate,
            on_failure=on_failure,
        )
        plan.tasks.extend(*tasks)
        return plan
