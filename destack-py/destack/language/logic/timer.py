from typing import TYPE_CHECKING

from destack.language.core import (
    Entity,
    Enum,
    EnumType,
    Event,
    NodeType,
    builtin_entity,
    builtin_enum,
    builtin_event,
    builtin_property,
)

from .schedule import Schedule

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@builtin_event(NodeType.TIMER_EVENT, is_abstract=True)
class TimerEvent(Event):
    """A TimerEvent is an Event that corresponds to a Timer."""

    timer: "Timer" = builtin_property(101)


@builtin_event(NodeType.TIMER_STARTED_EVENT)
class TimerStartedEvent(TimerEvent):
    """A Timer was started."""

    pass


@builtin_event(NodeType.TIMER_PAUSED_EVENT)
class TimerPausedEvent(TimerEvent):
    """A Timer was paused."""

    pass


@builtin_event(NodeType.TIMER_RESUMED_EVENT)
class TimerResumedEvent(TimerEvent):
    """A Timer was resumed."""

    pass


@builtin_event(NodeType.TIMER_COMPLETED_EVENT)
class TimerCompletedEvent(TimerEvent):
    """A Timer was completed."""

    pass


@builtin_event(NodeType.TIMER_CANCELLED_EVENT)
class TimerCancelledEvent(TimerEvent):
    """A Timer was cancelled."""

    pass


@builtin_enum(EnumType.TIMER_TYPE)
class TimerType(Enum):
    ONCE = 1
    RECURRING = 2


@builtin_entity(
    NodeType.TIMER,
    event_types=(
        NodeType.TIMER_STARTED_EVENT,
        NodeType.TIMER_COMPLETED_EVENT,
        NodeType.TIMER_CANCELLED_EVENT,
    ),
)
class Timer(Entity):
    """A Timer."""

    type: TimerType = builtin_property(100, is_repr=True)
    schedule: "Schedule | None" = builtin_property(110)
