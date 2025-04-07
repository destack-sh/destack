from typing import TYPE_CHECKING, Optional, Union
from uuid import UUID

from bench.language.core import (
    BuiltinEnum,
    EnumType,
    InlineNode,
    IsClaimable,
    IsInstantiable,
    IsModal,
    IsOwnable,
    IsTimed,
    IsTitled,
    IsTracked,
    LocalNodeList,
    NodeType,
    RunStatus,
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
    MANUAL = 10, "Manual", "Define manual Tasks to do", "fas fa-list"
    RUN = 20, "Run", "Sequence Tasks in a Run", "fas fa-list-ol"


@timed_node_(NodeType.PLAN)
class Plan(
    IsTimed,
    IsOwnable,
    IsClaimable,
    IsTracked,
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
    implemented_by: Optional["Run"] = p_internal(
        43, require=False, array=False, references=NodeType.RUN, same_bench=True
    )
    if TYPE_CHECKING:
        implemented_by_ptr: Optional[NodeReference] = None
        implemented_by_id: Optional[UUID] = None

    # status [80-90]

    triggers: LocalNodeList["Trigger"] = p_node_children(NodeType.TRIGGER)
    plans: LocalNodeList["Plan"] = p_node_children(NodeType.PLAN)
    tasks: LocalNodeList["Task"] = p_node_children(NodeType.TASK)

    def start(self) -> None:
        self.status = RunStatus.RUNNING
        self.started_at = self.active_session._oracle.utc()

    def complete(self) -> None:
        self.status = RunStatus.COMPLETED
        self.terminated_at = self.active_session._oracle.utc()
        if self.started_at is not None:
            self.duration = self.terminated_at - self.started_at

    def fail(self, error: "Error | None") -> None:
        self.error = error
        self.status = RunStatus.FAILED
        self.terminated_at = self.active_session._oracle.utc()
        if self.started_at is not None:
            self.duration = self.terminated_at - self.started_at

    @staticmethod
    def general(title: "TextLineIn", *tasks: "Task") -> "Plan":
        plan = Plan(type=PlanType.MANUAL, title=text_line(title))
        plan.tasks.extend(*tasks)
        return plan

    @staticmethod
    def flow(title: "TextLineIn", *tasks: "Task") -> "Plan":
        plan = Plan(type=PlanType.RUN, title=text_line(title))
        plan.tasks.extend(*tasks)
        return plan
