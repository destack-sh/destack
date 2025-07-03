from typing import TYPE_CHECKING

from destack.language.core import (
    Entity,
    Enum,
    EnumType,
    Event,
    IsSpatial,
    NodeType,
    builtin_enum,
    builtin_node,
    builtin_property,
)

from .schedule import Schedule

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.TIMER_EVENT, frozen=True, is_abstract=True)
class TimerEvent(Event["Timer"]):
    """A TimerEvent is an Event that corresponds to a Timer."""

    node: "Timer" = builtin_property(101)


@builtin_node(NodeType.TIMER_STARTED_EVENT, frozen=True)
class TimerStartedEvent(TimerEvent):
    """A Timer was started."""

    pass


@builtin_node(NodeType.TIMER_COMPLETED_EVENT, frozen=True)
class TimerCompletedEvent(TimerEvent):
    """A Timer was completed."""

    pass


@builtin_node(NodeType.TIMER_CANCELLED_EVENT, frozen=True)
class TimerCancelledEvent(TimerEvent):
    """A Timer was cancelled."""

    pass


@builtin_enum(EnumType.TIMER_TYPE)
class TimerType(Enum):
    ONCE = 1
    RECURRING = 2


@builtin_node(NodeType.TIMER)
class Timer(IsSpatial, Entity):
    """A Timer."""

    type: TimerType = builtin_property(100, is_repr=True)
    name: str = builtin_property(101, is_repr=True)
    schedule: "Schedule | None" = builtin_property(110)
