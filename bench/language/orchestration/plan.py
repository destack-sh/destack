from typing import TYPE_CHECKING, Union

from bench.language.core import (
    InlineNode,
    IsClaimable,
    IsInstantiable,
    IsModal,
    IsOwnable,
    IsProcessable,
    IsTimed,
    IsTitled,
    LocalNodeList,
    NodeType,
    ProcessStatus,
    TextLineIn,
    p_node_children,
    p_node_parent,
    text_line,
    timed_node_,
)
from bench.pb2 import PlanData

if TYPE_CHECKING:
    from bench.language import Error, Page, Run, Task, Thread, Trigger

# pyright: reportIncompatibleVariableOverride=false


@timed_node_(NodeType.PLAN)
class Plan(
    IsTimed,
    IsOwnable,
    IsClaimable,
    IsProcessable,
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

    # ...IsProcessable[80-]

    triggers: LocalNodeList["Trigger"] = p_node_children(NodeType.TRIGGER)
    plans: LocalNodeList["Plan"] = p_node_children(NodeType.PLAN)
    tasks: LocalNodeList["Task"] = p_node_children(NodeType.TASK)

    def start(self) -> None:
        self.status = ProcessStatus.RUNNING
        self.started_at = self.active_session._oracle.utc()

    def complete(self) -> None:
        self.status = ProcessStatus.COMPLETED
        self.terminated_at = self.active_session._oracle.utc()
        if self.started_at is not None:
            self.duration = self.terminated_at - self.started_at

    def fail(self, error: "Error | None") -> None:
        self.error = error
        self.status = ProcessStatus.FAILED
        self.terminated_at = self.active_session._oracle.utc()
        if self.started_at is not None:
            self.duration = self.terminated_at - self.started_at

    @staticmethod
    def new(title: "TextLineIn", *tasks: "Task") -> "Plan":
        plan = Plan(title=text_line(title))
        plan.tasks.extend(*tasks)
        return plan
