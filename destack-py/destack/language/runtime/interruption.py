from datetime import datetime, timedelta
from typing import TYPE_CHECKING, Optional

from fastuuid import UUID

from destack.language.core import (
    Analytic,
    BuiltinEnum,
    EnumType,
    Indexed,
    IsExtensible,
    IsRunnable,
    Node,
    NodeReference,
    NodeType,
    Particle,
    RunStatus,
    Spatial,
    enum_,
    node_,
    property_,
    property_parent_,
)
from destack.pb2 import InterruptionData

if TYPE_CHECKING:
    from destack.language import Message, Run, Span

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.INTERRUPTION_TYPE)
class InterruptionType(BuiltinEnum):
    PAUSE = 10, "Pause", "Run is marked as paused", "fas fa-pause"
    YIELD = 20, "Yield", "Yield to something", "fas fa-hand"
    WAIT = 30, "Wait", "Wait for a Trigger", "fas fa-hourglass-end"


RUN_STATUS_BY_INTERRUPTION_TYPE: dict[InterruptionType, RunStatus] = {
    InterruptionType.PAUSE: RunStatus.PAUSED,
    InterruptionType.YIELD: RunStatus.YIELDED,
    InterruptionType.WAIT: RunStatus.WAITING,
}
INTERRUPTION_TYPE_BY_RUN_STATUS: dict[RunStatus, InterruptionType] = {
    v: k for k, v in RUN_STATUS_BY_INTERRUPTION_TYPE.items()
}


@enum_(EnumType.INTERRUPTION_STATUS)
class InterruptionStatus(BuiltinEnum):
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
    Spatial,
    Particle,
    Analytic,
    Indexed,
    IsExtensible,
    Node[InterruptionData],
):
    """An Interruption in run of something."""

    # meta
    parent: Optional["Run"] = property_parent_(node_is_customizable=False)
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
        node_space_from="self",
    )

    # context
    # ...HasRuntimeContext[80-99]

    @property
    def is_open(self) -> bool:
        return self.status == InterruptionStatus.OPEN

    @property
    def is_closed(self) -> bool:
        return self.status == InterruptionStatus.COMPLETED
