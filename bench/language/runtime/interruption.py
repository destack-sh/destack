from datetime import datetime, timedelta
from typing import TYPE_CHECKING, Optional

from fastuuid import UUID

from bench.language.core import (
    BuiltinEnum,
    EnumType,
    IsExtensible,
    IsModal,
    NodeReference,
    NodeType,
    PackageNode,
    ProcessStatus,
    Runnable,
    enum_,
    node_,
    p_internal,
    p_node_ancestor,
    p_node_parent,
    p_regular,
)
from bench.pb2 import InterruptionData

if TYPE_CHECKING:
    from bench.language import (
        Action,
        Flow,
        FlowEdge,
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
class Interruption(IsModal, IsExtensible, PackageNode[InterruptionData]):
    """An Interruption in the processing or execution of something."""

    # meta
    parent: Optional["Run"] = p_node_parent(4, NodeType.RUN)
    type: InterruptionType = p_regular(30)
    root: "Run | None" = p_node_ancestor(
        31, NodeType.RUN, require=False, store=True, wire=True, is_bench_implicit=True
    )
    flow: Optional["Flow"] = p_internal(33)
    action: Optional["Action"] = p_internal(34)
    link: Optional["FlowEdge"] = p_internal(35)
    if TYPE_CHECKING:
        flow_ptr: Optional[NodeReference] = None
        action_ptr: Optional[NodeReference] = None
        link_ptr: Optional[NodeReference] = None
    span: Optional["Span"] = p_internal(37)
    if TYPE_CHECKING:
        root_id: Optional[UUID] = None
        root_ptr: Optional[NodeReference] = None
        page_id: Optional[UUID] = None
        page_ptr: Optional[NodeReference] = None
        flow_id: Optional[UUID] = None
        flow_ptr: Optional[NodeReference] = None
        action_id: Optional[UUID] = None
        action_ptr: Optional[NodeReference] = None
        link_id: Optional[UUID] = None
        link_ptr: Optional[NodeReference] = None
        span_id: Optional[UUID] = None
        span_ptr: Optional[NodeReference] = None

    # status
    status: InterruptionStatus = p_regular(40, default=InterruptionStatus.OPEN)
    duration: Optional[timedelta] = p_internal(41)
    closed_at: Optional[datetime] = p_internal(42)

    # content
    response: Optional[InterruptionResponse] = p_internal(54)
    message: Optional["Message"] = p_regular(
        55,
        baseless=True,
        description="The Message that was created for this Interruption.",
        same_bench=True,
    )
    task: Optional["Task"] = p_regular(
        56,
        description="The Task that was created for this Interruption.",
        same_bench=True,
    )

    # context
    # ...HasRuntimeContext[80-99]

    def __content_str__(self):
        node = self.runnable
        path = node.absolute_path if node else "???"
        if self.duration is not None:
            duration_str = f"{self.duration.total_seconds():.3f}s"
            return (
                f"{self.type.bench_name}:{path}, {self.status.bench_name}, duration={duration_str}"
            )
        else:
            return f"{self.type.bench_name}:{path}, {self.status.bench_name}"

    @property
    def runnable(self):
        if self.link_ptr:
            return self.link
        elif self.action_ptr:
            return self.action
        else:
            return self.flow

    @property
    def is_open(self) -> bool:
        return self.status == InterruptionStatus.OPEN

    @property
    def is_closed(self) -> bool:
        return self.status == InterruptionStatus.COMPLETED

    def is_in(self, *nodes: Runnable) -> bool:
        """Whether the Interruption is a descendant of a Run of any of the given Nodes."""
        if (parent := self.parent) is None:
            return False
        else:
            return parent.is_in(*nodes)

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
            flow=run.flow,
            action=run.action,
            span=span,
            mode=run.mode,
        )
