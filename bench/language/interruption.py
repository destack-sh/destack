from datetime import datetime, timedelta
from typing import Any, Optional, cast

from git import TYPE_CHECKING

from bench.language.const import EnumType, NodeType, ObjectKind, RunStatus, StructType, enum_
from bench.language.field import TypeBase
from bench.language.node import NodeReference, RuntimeNode, Struct, struct_, timed_node_
from bench.language.property import (
    p_internal,
    p_node_ancestor,
    p_node_parent,
    p_regular,
    p_value_packed,
    p_value_runtime,
)
from bench.language.trigger import Trigger
from bench.language.value import CustomObject
from bench.proto.wire.lang_pb2 import InterruptionData
from bench.utils.func import IdEnum

if TYPE_CHECKING:
    from bench.language import Action, Block, Pipe, Run

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.BREAKPOINT_SITE)
class BreakpointSite(IdEnum):
    # run
    RUN_BEFORE = 1
    RUN_AFTER_FAILED = 2
    RUN_AFTER_COMPLETED = 3
    RUN_AFTER = 5
    # flow
    ...
    # action
    ...


@enum_(EnumType.BREAKPOINT_TARGET)
class BreakpointScope(IdEnum):
    # general
    SELF = 1
    CHILD = 2
    # DESCENDANT, ...?
    # flow
    ACTION = 20
    PIPE = 21


@enum_(EnumType.BREAKPOINT_ACTION)
class BreakpointAction(IdEnum):
    YIELD = 1
    # LOG, FAIL, ...?


@struct_(StructType.BREAKPOINT)
class Breakpoint(Struct):
    """A (conditional) Breakpoint for some Run. Overlapping Breakpoints coalesce."""

    site: BreakpointSite = p_regular(30)
    scope: BreakpointScope = p_regular(31, default=BreakpointScope.SELF)
    action: BreakpointAction = p_regular(32, default=BreakpointAction.YIELD)

    def __content_str__(self) -> str:
        return f"{self.site.bench_name}:{self.scope.bench_name} -> {self.action.bench_name}"

    @staticmethod
    def before(
        scope: BreakpointScope = BreakpointScope.SELF,
        action: BreakpointAction = BreakpointAction.YIELD,
    ) -> "Breakpoint":
        return Breakpoint(site=BreakpointSite.RUN_BEFORE, scope=scope, action=action)

    @staticmethod
    def after(
        scope: BreakpointScope = BreakpointScope.SELF,
        action: BreakpointAction = BreakpointAction.YIELD,
    ) -> "Breakpoint":
        return Breakpoint(site=BreakpointSite.RUN_AFTER, scope=scope, action=action)

    @staticmethod
    def after_failed(
        scope: BreakpointScope = BreakpointScope.SELF,
        action: BreakpointAction = BreakpointAction.YIELD,
    ) -> "Breakpoint":
        return Breakpoint(site=BreakpointSite.RUN_AFTER_FAILED, scope=scope, action=action)

    @staticmethod
    def after_completed(
        scope: BreakpointScope = BreakpointScope.SELF,
        action: BreakpointAction = BreakpointAction.YIELD,
    ) -> "Breakpoint":
        return Breakpoint(site=BreakpointSite.RUN_AFTER_COMPLETED, scope=scope, action=action)


@enum_(EnumType.INTERRUPTION_TYPE)
class InterruptionType(IdEnum):
    PAUSE = 1  # external pause
    YIELD = 2  # voluntary yield
    WAIT = 3  # wait on trigger


RUN_STATUS_BY_INTERRUPTION_TYPE: dict[InterruptionType, RunStatus] = {
    InterruptionType.PAUSE: RunStatus.PAUSED,
    InterruptionType.YIELD: RunStatus.YIELDED,
    InterruptionType.WAIT: RunStatus.WAITING,
}
INTERRUPTION_TYPE_BY_RUN_STATUS: dict[RunStatus, InterruptionType] = {
    v: k for k, v in RUN_STATUS_BY_INTERRUPTION_TYPE.items()
}


@enum_(EnumType.INTERRUPTION_STATUS)
class InterruptionStatus(IdEnum):
    OPEN = 1
    CANCELLED = 7
    COMPLETED = 10

    @property
    def is_open(self) -> bool:
        return self < 7

    @property
    def is_closed(self) -> bool:
        return self >= 7


@timed_node_(NodeType.INTERRUPTION)
class Interruption(RuntimeNode[InterruptionData]):
    """An Interruption in the execution of a Run."""

    # meta
    parent: "Run" = p_node_parent(4, NodeType.RUN)
    type: InterruptionType = p_regular(30, require=True)
    root: "Run | None" = p_node_ancestor(
        31, NodeType.RUN, require=False, store=True, wire=True, is_bench_implicit=True
    )
    if TYPE_CHECKING:
        root_ptr: Optional[NodeReference] = None
    block: Optional["Block"] = p_internal(32, require=False, array=False, references=NodeType.BLOCK)
    action: Optional["Action"] = p_internal(
        33, require=False, array=False, references=NodeType.ACTION
    )
    pipe: Optional["Pipe"] = p_internal(34, require=False, array=False, references=NodeType.PIPE)
    if TYPE_CHECKING:
        block_ptr: Optional[NodeReference] = None
        action_ptr: Optional[NodeReference] = None
        pipe_ptr: Optional[NodeReference] = None
    attempt_no: Optional[int] = p_internal(37, require=False, default=None)
    breakpoint_site: BreakpointSite | None = p_internal(38)

    # status
    status: InterruptionStatus = p_internal(40, default=InterruptionStatus.OPEN)
    duration: Optional[timedelta] = p_internal(41, require=False, default=None)
    closed_at: Optional[datetime] = p_internal(42, require=False, default=None)
    trigger: Optional["Trigger"] = p_regular(
        49, require=False, default=None, references=NodeType.TRIGGER
    )

    # content
    inputs_packed: Any = p_value_packed(51)
    inputs: Any = p_value_runtime(51, kind=ObjectKind.INPUT, typ=None)
    outputs_packed: Any = p_value_packed(52)
    outputs: Any = p_value_runtime(
        52, kind=ObjectKind.OUTPUT, typ=lambda self: cast("Interruption", self).output_type
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
        if self.pipe_ptr:
            return self.pipe
        elif self.action_ptr:
            return self.action
        else:
            return self.block

    @property
    def output_type(self) -> TypeBase | None:
        runnable = self.runnable
        if runnable is None:
            return None
        return runnable.output_type

    @property
    def is_open(self) -> bool:
        return self.status == InterruptionStatus.OPEN

    @property
    def is_closed(self) -> bool:
        return self.status == InterruptionStatus.COMPLETED

    def complete(self, outputs: CustomObject | None = None, _trigger_runtime: bool = True) -> None:
        """Mark this Interrupt as closed."""
        assert not self.is_closed, f"{self!r} is already closed"
        assert self._session is not None, f"{self!r} has no session"
        self.status = InterruptionStatus.COMPLETED
        self.closed_at = self._session._oracle.utc()
        self.duration = self.closed_at - self.created_at
        self.outputs = outputs
        runtime = self._session.runtime
        if runtime and _trigger_runtime:
            runs_to_resume = runtime.get_interrupted_runs(self._graph, self)
            runtime.resume(*runs_to_resume)

    def cancel(self, _trigger_runtime: bool = True) -> None:
        """Mark this Interrupt as cancelled."""
        assert not self.is_closed, f"{self!r} is already closed"
        assert self._session is not None, f"{self!r} has no session"
        self.status = InterruptionStatus.CANCELLED
        self.closed_at = self._session._oracle.utc()
        self.duration = self.closed_at - self.created_at
        runtime = self._session.runtime
        if runtime and _trigger_runtime:
            runs_to_resume = runtime.get_interrupted_runs(self._graph, self)
            runtime.resume(*runs_to_resume)

    @staticmethod
    def from_run(
        kind: InterruptionType,
        run: "Run",
        trigger: "Trigger | None" = None,
        attempt: Optional[int] = None,
        breakpoint: BreakpointSite | None = None,
    ) -> "Interruption":
        return Interruption(
            type=kind,
            parent=run,
            session=run.session,
            block=run.block,
            action=run.action,
            pipe=run.pipe,
            trigger=trigger,
            attempt_no=attempt,
            breakpoint_site=breakpoint,
            mode=run.mode,
        )
