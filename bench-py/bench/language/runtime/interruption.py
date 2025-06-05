from datetime import datetime, timedelta
from typing import TYPE_CHECKING, Optional

from fastuuid import UUID

from bench.language.core import (
    BuiltinEnum,
    EnumType,
    HasEnvironment,
    IsExtensible,
    IsInPackage,
    IsRunnable,
    Node,
    NodeReference,
    NodeType,
    ProcessStatus,
    enum_,
    node_,
    property_,
    property_parent_,
)
from bench.pb2 import InterruptionData

if TYPE_CHECKING:
    from bench.language import (
        Message,
        Run,
        Span,
        Task,
    )

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.INTERRUPTION_TYPE)
class InterruptionType(BuiltinEnum):
    PAUSE = 10, "Pause", "Run is marked as paused", "fas fa-pause"
    YIELD = 20, "Yield", "Yield to something", "fas fa-hand"
    WAIT = 30, "Wait", "Wait for a Trigger", "fas fa-hourglass-end"


PROCESS_STATUS_BY_INTERRUPTION_TYPE: dict[InterruptionType, ProcessStatus] = {
    InterruptionType.PAUSE: ProcessStatus.PAUSED,
    InterruptionType.YIELD: ProcessStatus.YIELDED,
    InterruptionType.WAIT: ProcessStatus.WAITING,
}
INTERRUPTION_TYPE_BY_PROCESS_STATUS: dict[ProcessStatus, InterruptionType] = {
    v: k for k, v in PROCESS_STATUS_BY_INTERRUPTION_TYPE.items()
}


@enum_(EnumType.INTERRUPTION_STATUS)
class InterruptionStatus(BuiltinEnum):  # NOTE: see RunStatus
    OPEN = 10
    CANCELLED = 30
    COMPLETED = 33

    @property
    def is_open(self) -> bool:
        return self > 10 and self < 30

    @property
    def is_closed(self) -> bool:
        return self >= 30


@enum_(EnumType.INTERRUPTION_RESPONSE)
class InterruptionResponse(BuiltinEnum):
    ACCEPT = 10
    REJECT = 20
    # CRITIQUE/EDIT, ...?


@node_(NodeType.INTERRUPTION)
class Interruption(
    HasEnvironment,
    IsExtensible,
    IsInPackage,
    Node[InterruptionData],
):
    """An Interruption in the processing or execution of something."""

    # meta
    parent: Optional["Run"] = property_parent_()
    type: InterruptionType = property_(30)
    runnable: Optional["IsRunnable"] = property_(32)
    span: Optional["Span"] = property_(37)
    if TYPE_CHECKING:
        runnable_ptr: Optional[NodeReference] = None
        runnable_id: Optional[UUID] = None
        span_id: Optional[UUID] = None
        span_ptr: Optional[NodeReference] = None

    # status
    status: InterruptionStatus = property_(40, default=InterruptionStatus.OPEN)
    duration: Optional[timedelta] = property_(41)
    closed_at: Optional[datetime] = property_(42)

    # content
    response: Optional[InterruptionResponse] = property_(54)
    message: Optional["Message"] = property_(
        55,
        description="The Message that was created for this Interruption.",
        node_bench_from="self",
    )
    task: Optional["Task"] = property_(
        56,
        description="The Task that was created for this Interruption.",
        node_bench_from="self",
    )

    # context
    # ...HasRuntimeContext[80-99]

    @property
    def is_open(self) -> bool:
        return self.status == InterruptionStatus.OPEN

    @property
    def is_closed(self) -> bool:
        return self.status == InterruptionStatus.COMPLETED

    def complete(self, _trigger_runtime: bool = True) -> None:
        """Mark this Interrupt as closed."""
        assert not self.is_closed, f"{self!r} is already closed"
        assert self._session is not None, f"{self!r} has no session"
        self.status = InterruptionStatus.COMPLETED
        self.closed_at = self._session.oracle.utc()
        self.duration = self.closed_at - self.created_at
        runtime = self._session.runtime
        if runtime and _trigger_runtime:
            runs_to_resume = runtime.get_interrupted_runs(self._graph, self)
            runtime.resume_run(*runs_to_resume)

    def cancel(self, _trigger_runtime: bool = True) -> None:
        """Mark this Interrupt as cancelled."""
        assert not self.is_closed, f"{self!r} is already closed"
        assert self._session is not None, f"{self!r} has no session"
        self.status = InterruptionStatus.CANCELLED
        self.closed_at = self._session.oracle.utc()
        self.duration = self.closed_at - self.created_at
        runtime = self._session.runtime
        if runtime and _trigger_runtime:
            runs_to_resume = runtime.get_interrupted_runs(self._graph, self)
            runtime.resume_run(*runs_to_resume)

    @staticmethod
    def from_run(
        kind: InterruptionType,
        run: "Run",
        span: Optional["Span"] = None,
    ) -> "Interruption":
        return Interruption(
            type=kind,
            parent=run,
            runnable=run.runnable,
            span=span,
            environment_type=run.environment_type,
        )
