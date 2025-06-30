from typing import TYPE_CHECKING

from destack.language.core import (
    Entity,
    Enum,
    EnumType,
    Event,
    HasName,
    IsSpatial,
    NodeType,
    builtin_enum,
    builtin_node,
    property_,
)

from .schedule import Schedule

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.TIMER_EVENT, is_abstract=True)
class TimerEvent(Event["Timer"]):
    """A TimerEvent is an Event that corresponds to a Timer."""

    node: "Timer" = property_(35)


@builtin_node(NodeType.TIMER_STARTED_EVENT)
class TimerStartedEvent(TimerEvent):
    """A Timer was started."""

    pass


@builtin_node(NodeType.TIMER_COMPLETED_EVENT)
class TimerCompletedEvent(TimerEvent):
    """A Timer was completed."""

    pass


@builtin_node(NodeType.TIMER_CANCELLED_EVENT)
class TimerCancelledEvent(TimerEvent):
    """A Timer was cancelled."""

    pass


@builtin_enum(EnumType.TIMER_TYPE)
class TimerType(Enum):
    ONCE = 1
    RECURRING = 2


@builtin_node(NodeType.TIMER)
class Timer(IsSpatial, HasName, Entity):
    """A Timer."""

    type: TimerType = property_(30)
    schedule: "Schedule | None" = property_(40)
