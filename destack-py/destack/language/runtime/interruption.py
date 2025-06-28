from datetime import datetime, timedelta
from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Analytic,
    Enum,
    EnumType,
    IsExtensible,
    IsRunnable,
    Node,
    NodeReference,
    NodeType,
    Particle,
    Spatial,
    builtin_enum,
    builtin_node,
    property_,
    property_parent_,
)
from destack.proto import InterruptionProto

if TYPE_CHECKING:
    from destack.language import Message, Run, Span

# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.INTERRUPTION_TYPE)
class InterruptionType(Enum):
    PAUSE = 10, "Pause", "Run is marked as paused", "fas fa-pause"
    YIELD = 20, "Yield", "Yield to something", "fas fa-hand"
    WAIT = 30, "Wait", "Wait for a Trigger", "fas fa-hourglass-end"


@builtin_enum(EnumType.INTERRUPTION_STATUS)
class InterruptionStatus(Enum):
    OPEN = 10
    CANCELLED = 30
    COMPLETED = 33

    @property
    def is_open(self) -> bool:
        return self > 10 and self < 30

    @property
    def is_closed(self) -> bool:
        return self >= 30


@builtin_enum(EnumType.INTERRUPTION_RESPONSE)
class InterruptionResponse(Enum):
    ACCEPT = 10
    REJECT = 20
    # CRITIQUE/EDIT, ...?


@builtin_node(NodeType.INTERRUPTION)
class Interruption(
    Spatial,
    Particle,
    Analytic,
    IsExtensible,
    Node[InterruptionProto],
):
    """An Interruption in run of something."""

    # meta
    parent: Optional["Run"] = property_parent_(node_is_customizable=False)
    type: InterruptionType = property_(30)
    runnable: Optional["IsRunnable"] = property_(32)
    span: Optional["Span"] = property_(37)
    if TYPE_CHECKING:
        runnable_ptr: Optional[NodeReference] = None
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
