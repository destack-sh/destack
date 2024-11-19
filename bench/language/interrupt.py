from datetime import datetime, timedelta
from typing import Any, Optional

from git import TYPE_CHECKING

from bench.language.const import EnumType, NodeType, ObjectKind, RunStatus, StructType, enum_
from bench.language.node import NodeReference, RuntimeNode, Struct, struct_, timed_node_
from bench.language.property import (
    p_internal,
    p_node_ancestor,
    p_node_parent,
    p_regular,
    p_value_packed,
    p_value_runtime,
)
from bench.language.session import HasSessionContext
from bench.language.trigger import Trigger
from bench.language.value import CustomObject
from bench.utils.func import IdEnum

if TYPE_CHECKING:
    from bench.language import Block, Pipe, Run, Step

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
    STEP = 20


@enum_(EnumType.BREAKPOINT_ACTION)
class BreakpointAction(IdEnum):
    YIELD = 1
    # LOG, FAIL, ...?


@struct_(StructType.BREAKPOINT)
class Breakpoint(Struct):
    """A (conditional) Breakpoint for some Run."""

    site: BreakpointSite = p_regular(30)
    scope: BreakpointScope = p_regular(31, default=BreakpointScope.SELF)
    action: BreakpointAction = p_regular(32, default=BreakpointAction.YIELD)

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


@enum_(EnumType.INTERRUPT_KIND)
class InterruptKind(IdEnum):
    PAUSE = 1  # external pause
    YIELD = 2  # voluntary yield
    WAIT = 3  # wait on trigger


RUN_STATUS_BY_INTERRUPT_KIND: dict[InterruptKind, RunStatus] = {
    InterruptKind.PAUSE: RunStatus.PAUSED,
    InterruptKind.YIELD: RunStatus.YIELDED,
    InterruptKind.WAIT: RunStatus.WAITING,
}
INTERRUPT_KIND_BY_RUN_STATUS: dict[RunStatus, InterruptKind] = {
    v: k for k, v in RUN_STATUS_BY_INTERRUPT_KIND.items()
}


@enum_(EnumType.INTERRUPT_STATUS)
class InterruptStatus(IdEnum):
    OPEN = 1
    CLOSED = 2


@timed_node_(NodeType.INTERRUPT)
class Interrupt(RuntimeNode, HasSessionContext):
    """An Interrupt in the execution of a Run."""

    # meta
    parent: "Run" = p_node_parent(4, NodeType.RUN)
    kind: InterruptKind = p_regular(30, require=True)
    root: "Run | None" = p_node_ancestor(
        31, NodeType.RUN, require=False, store=True, wire=True, is_bench_implicit=True
    )
    if TYPE_CHECKING:
        root_ptr: Optional[NodeReference] = None
    block: Optional["Block"] = p_internal(32, require=False, array=False, references=NodeType.BLOCK)
    step: Optional["Step"] = p_internal(33, require=False, array=False, references=NodeType.STEP)
    pipe: Optional["Pipe"] = p_internal(34, require=False, array=False, references=NodeType.PIPE)
    attempt_no: Optional[int] = p_internal(37, require=False, default=None)
    breakpoint_site: BreakpointSite | None = p_internal(38)

    # status
    status: InterruptStatus = p_internal(40, default=InterruptStatus.OPEN)
    duration: Optional[timedelta] = p_internal(41, require=False, default=None)
    closed_at: Optional[datetime] = p_internal(42, require=False, default=None)
    trigger: Optional["Trigger"] = p_regular(
        49, require=False, default=None, references=NodeType.TRIGGER
    )

    # content
    outputs_packed: Any = p_value_packed(50)
    outputs: Any = p_value_runtime(50, kind=ObjectKind.OUTPUT, typ=None)

    def close(self, outputs: CustomObject | None = None) -> None:
        """Mark this Interrupt as closed."""
        if self.status == InterruptStatus.CLOSED:
            return
        assert self._session is not None, f"{self!r} has no session"
        self.status = InterruptStatus.CLOSED
        self.closed_at = self._session._oracle.utc()
        self.duration = self.closed_at - self.created_at
        self.outputs = outputs

    @property
    def is_open(self) -> bool:
        return self.status == InterruptStatus.OPEN

    @property
    def is_closed(self) -> bool:
        return self.status == InterruptStatus.CLOSED

    @staticmethod
    def from_run(
        kind: InterruptKind,
        run: "Run",
        trigger: "Trigger | None" = None,
        attempt: Optional[int] = None,
        breakpoint: BreakpointSite | None = None,
    ) -> "Interrupt":
        return Interrupt(
            kind=kind,
            parent=run,
            session=run.session,
            block=run.block,
            step=run.step,
            pipe=run.pipe,
            trigger=trigger,
            attempt_no=attempt,
            breakpoint_site=breakpoint,
        )
