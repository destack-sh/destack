from typing import TYPE_CHECKING, Optional, Union
from uuid import UUID

from bench.language.core import (
    BuiltinEnum,
    ColorType,
    EnumType,
    InlineNode,
    IsClaimable,
    IsInstantiable,
    IsModal,
    IsOwnable,
    IsRuntimeControllable,
    IsTimed,
    IsTitled,
    LocalNodeList,
    NodeType,
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
    from bench.language import Error, NodeReference, Page, Run, Task, Thread, Trigger

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.PLAN_TYPE)
class PlanType(BuiltinEnum):
    GENERAL = 10, "Generic", "Define general Tasks to do", "fas fa-list"
    FLOW = 20, "Flow", "Sequence Tasks in a Flow", "fas fa-list-ol"


@enum_(EnumType.PLAN_STATUS)
class PlanStatus(BuiltinEnum):
    # pre
    CREATED = 1, None, None, "fas fa-clock", ColorType.GRAY
    # active
    RUNNING = 10, None, None, "fas fa-circle-notch", ColorType.BLUE
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


@enum_(EnumType.PLAN_FAILURE_MODE)
class PlanFailureMode(BuiltinEnum):
    FAIL = 10, "Fail", "Fail the Plan"
    END = 20, "Complete", "Complete the Plan"
    CONTINUE = 30, "Continue", "Continue the Plan (skip failures)"


@timed_node_(NodeType.PLAN)
class Plan(
    IsTimed,
    IsOwnable,
    IsClaimable,
    IsRuntimeControllable,
    IsModal,
    IsInstantiable,
    IsTitled,
    InlineNode[PlanData],
):
    """A Plan for something expressed as a sequence of Tasks."""

    # meta
    parent: Union["Page", "Thread", "Plan", "Run", None] = p_node_parent(
        4, NodeType.PAGE, NodeType.THREAD, NodeType.PLAN, NodeType.RUN
    )
    type: PlanType = p_regular(30)
    on_failure: "PlanFailureMode" = p_internal(42)
    implemented_by: Optional["Run"] = p_internal(
        43, require=False, array=False, references=NodeType.RUN, same_bench=True
    )
    if TYPE_CHECKING:
        implemented_by_ptr: Optional[NodeReference] = None
        implemented_by_id: Optional[UUID] = None

    # status [80-90]
    status: PlanStatus = p_regular(80, default=PlanStatus.CREATED)

    triggers: LocalNodeList["Trigger"] = p_node_children(NodeType.TRIGGER)
    plans: LocalNodeList["Plan"] = p_node_children(NodeType.PLAN)
    tasks: LocalNodeList["Task"] = p_node_children(NodeType.TASK)

    def start(self) -> None:
        self.status = PlanStatus.RUNNING
        self.started_at = self.active_session._oracle.utc()

    def complete(self) -> None:
        self.status = PlanStatus.COMPLETED
        self.terminated_at = self.active_session._oracle.utc()
        if self.started_at is not None:
            self.duration = self.terminated_at - self.started_at

    def fail(self, error: "Error | None") -> None:
        self.error = error
        self.status = PlanStatus.FAILED
        self.terminated_at = self.active_session._oracle.utc()
        if self.started_at is not None:
            self.duration = self.terminated_at - self.started_at

    @staticmethod
    def general(
        title: "TextLineIn",
        *tasks: "Task",
        on_failure: PlanFailureMode = PlanFailureMode.END,
    ) -> "Plan":
        plan = Plan(type=PlanType.GENERAL, title=text_line(title), on_failure=on_failure)
        plan.tasks.extend(*tasks)
        return plan

    @staticmethod
    def flow(
        title: "TextLineIn",
        *tasks: "Task",
        on_failure: PlanFailureMode = PlanFailureMode.END,
    ) -> "Plan":
        plan = Plan(type=PlanType.FLOW, title=text_line(title), on_failure=on_failure)
        plan.tasks.extend(*tasks)
        return plan
