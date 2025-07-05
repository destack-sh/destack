from datetime import datetime, timedelta
from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Entity,
    Enum,
    EnumType,
    IsRunnable,
    IsSpatial,
    NodeReference,
    NodeType,
    builtin_enum,
    builtin_node,
    builtin_property,
    builtin_property_parent,
)

if TYPE_CHECKING:
    from destack.language import Message, Run, SpanEvent

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
class Interruption(IsSpatial, Entity):
    """An Interruption in run of something."""

    # meta
    parent: Optional["Run"] = builtin_property_parent()
    type: InterruptionType = builtin_property(100)
    runnable: Optional["IsRunnable"] = builtin_property(110)
    span: Optional["SpanEvent"] = builtin_property(111)
    if TYPE_CHECKING:
        runnable_ptr: Optional[NodeReference] = None
        span_ptr: Optional[NodeReference] = None

    # status
    status: InterruptionStatus = builtin_property(120, default=InterruptionStatus.OPEN)
    duration: Optional[timedelta] = builtin_property(121)
    closed_at: Optional[datetime] = builtin_property(122)

    # content
    response: Optional[InterruptionResponse] = builtin_property(130)
    message: Optional["Message"] = builtin_property(
        131,
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
