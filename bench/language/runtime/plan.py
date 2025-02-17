from typing import TYPE_CHECKING, Optional, Union
from uuid import UUID

from bench.language.core import (
    BuiltinEnum,
    EnumType,
    NodeType,
    RunStatus,
    RuntimeNode,
    StructType,
    Text,
    enum_,
    p_internal,
    p_node_parent,
    p_regular,
    timed_node_,
)
from bench.pb2 import PlanData

if TYPE_CHECKING:
    from bench.language import (
        Call,
        CallExecutionMode,
        CallFailureMode,
        CallPlan,
        CallTerminationMode,
        Error,
        NodeReference,
        Run,
        Text,
    )

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.PLAN_TYPE)
class PlanType(BuiltinEnum):
    CALL = 1, "Run"


@timed_node_(NodeType.PLAN)
class Plan(RuntimeNode[PlanData]):
    """A Plan for a sequence of Runs."""

    # meta
    parent: Union["Run", None] = p_node_parent(4, NodeType.RUN)
    type: PlanType = p_regular(30)
    execution: "CallExecutionMode" = p_internal(31)
    on_terminate: "CallTerminationMode" = p_internal(33)
    on_error: "CallFailureMode" = p_internal(34)

    # status
    status: RunStatus = p_regular(40, default=RunStatus.SCHEDULED)
    started_by: "Run" = p_internal(41, require=False, array=False, references=NodeType.RUN)
    terminated_by: Optional["Run"] = p_internal(
        42, require=False, array=False, references=NodeType.RUN
    )
    error: Optional["Error"] = p_internal(53, require=False, array=False, struct=StructType.ERROR)
    if TYPE_CHECKING:
        started_by_ptr: Optional[NodeReference] = None
        started_by_id: Optional[UUID] = None
        terminated_by_ptr: Optional[NodeReference] = None
        terminated_by_id: Optional[UUID] = None

    # content
    title: str | None = p_regular(50, default=None)
    text: Optional["Text"] = p_regular(51, default=None, struct=StructType.TEXT)
    calls: list["Call"] = p_internal(52, require=True, array=True, struct=StructType.CALL)
    step: int | None = p_internal(55)

    def complete(self, by: "Run") -> None:
        self.terminated_by = by
        self.status = RunStatus.COMPLETED

    def fail(self, by: "Run") -> None:
        self.terminated_by = by
        self.error = by.error
        self.status = RunStatus.FAILED

    @staticmethod
    def from_call(
        run: "Run",
        call_plan: "CallPlan",
        status: RunStatus = RunStatus.SCHEDULED,
    ) -> "Plan":
        return Plan(
            parent=run,
            type=PlanType.CALL,
            status=status,
            started_by=run,
            execution=call_plan.execution,
            on_terminate=call_plan.on_terminate,
            on_error=call_plan.on_error,
            calls=call_plan.calls,
            _skip_validate_self=True,
        )
