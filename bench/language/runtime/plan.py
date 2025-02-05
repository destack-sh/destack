from typing import TYPE_CHECKING, Optional, Union
from uuid import UUID

from bench.language.core import (
    NodeType,
    RunStatus,
    RuntimeNode,
    StructType,
    Text,
    p_internal,
    p_node_parent,
    p_regular,
    timed_node_,
)
from bench.pb2.lang_pb2 import RunPlanData

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


@timed_node_(NodeType.RUN_PLAN)
class RunPlan(RuntimeNode[RunPlanData]):
    """A RunPlan is a plan for a sequence of Runs."""

    # NOTE :Architecture: maybe RunPlan should be just Plan?

    # meta
    parent: Union["Run", None] = p_node_parent(4, NodeType.RUN)
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
    def new(
        run: "Run",
        call_plan: "CallPlan",
        status: RunStatus = RunStatus.SCHEDULED,
    ) -> "RunPlan":
        return RunPlan(
            parent=run,
            status=status,
            started_by=run,
            execution=call_plan.execution,
            on_terminate=call_plan.on_terminate,
            on_error=call_plan.on_error,
            calls=call_plan.calls,
            _skip_validate_self=True,
        )
