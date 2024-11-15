from datetime import datetime, timedelta
from typing import Any, Optional

from git import TYPE_CHECKING

from bench.language.const import EnumType, NodeType, ObjectKind, StructType, enum_
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
from bench.utils.func import IdEnum

if TYPE_CHECKING:
    from bench.language import Block, Expression, Pipe, Run, Step

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.BREAKPOINT_KIND)
class BreakpointKind(IdEnum):
    # basic
    START_RUN = 1
    FAIL_RUN = 2
    COMPLETE_RUN = 3
    RETRY_RUN = 4
    FAIL_ATTEMPT = 5
    # flow
    ...


@enum_(EnumType.BREAKPOINT_SCOPE)
class BreakpointScope(IdEnum):
    SELF = 1


@enum_(EnumType.BREAKPOINT_ACTION)
class BreakpointAction(IdEnum):
    YIELD = 1
    LOG = 2


@struct_(StructType.BREAKPOINT)
class Breakpoint(Struct):
    """A (conditional) Breakpoint for some Run."""

    kind: BreakpointKind = p_regular(30)
    scope: BreakpointScope = p_regular(31, default=BreakpointScope.SELF)
    action: BreakpointAction = p_regular(32, default=BreakpointAction.YIELD)

    condition: Optional["Expression"] = p_regular(
        40, require=False, array=False, struct=StructType.EXPRESSION
    )


@enum_(EnumType.INTERRUPT_KIND)
class InterruptKind(IdEnum):
    PAUSE = 1
    YIELD = 2
    TRIGGER = 3


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
    trigger: Optional["Trigger"] = p_regular(
        37, require=False, default=None, references=NodeType.TRIGGER
    )
    attempt_no: Optional[int] = p_internal(38, require=False, default=None)

    # status
    status: InterruptStatus = p_internal(40, default=InterruptStatus.OPEN)
    duration: Optional[timedelta] = p_internal(41, require=False, default=None)
    opened_at: Optional[datetime] = p_regular(42, require=False, default=None)
    closed_at: Optional[datetime] = p_regular(43, require=False, default=None)

    # content
    outputs_packed: Any = p_value_packed(50)
    outputs: Any = p_value_runtime(50, kind=ObjectKind.OUTPUT, typ=None)

    @staticmethod
    def from_yield(run: "Run") -> "Interrupt":
        return Interrupt(
            kind=InterruptKind.YIELD,
            parent=run,
            session=run.session,
            block=run.block,
            step=run.step,
            pipe=run.pipe,
        )
