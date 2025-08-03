from typing import TYPE_CHECKING

from destack.core import (
    Entity,
    EnumDeclaration,
    EnumType,
    Event,
    NodeType,
    declare_entity,
    declare_enum,
    declare_event,
    declare_property,
)

from .schedule import Schedule

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@declare_event(NodeType.TIMER_EVENT, is_abstract=True)
class TimerEvent(Event):
    """A TimerEvent is an Event that corresponds to a Timer."""

    timer: "Timer" = declare_property(101)


@declare_event(NodeType.TIMER_STARTED_EVENT)
class TimerStartedEvent(TimerEvent):
    """A Timer was started."""

    pass


@declare_event(NodeType.TIMER_PAUSED_EVENT)
class TimerPausedEvent(TimerEvent):
    """A Timer was paused."""

    pass


@declare_event(NodeType.TIMER_RESUMED_EVENT)
class TimerResumedEvent(TimerEvent):
    """A Timer was resumed."""

    pass


@declare_event(NodeType.TIMER_COMPLETED_EVENT)
class TimerCompletedEvent(TimerEvent):
    """A Timer was completed."""

    pass


@declare_event(NodeType.TIMER_CANCELLED_EVENT)
class TimerCancelledEvent(TimerEvent):
    """A Timer was cancelled."""

    pass


@declare_enum(EnumType.TIMER_TYPE)
class TimerType(EnumDeclaration):
    ONCE = 1
    RECURRING = 2


@declare_entity(
    NodeType.TIMER,
    event_types=(
        NodeType.TIMER_STARTED_EVENT,
        NodeType.TIMER_COMPLETED_EVENT,
        NodeType.TIMER_CANCELLED_EVENT,
    ),
)
class Timer(Entity):
    """A Timer."""

    type: TimerType = declare_property(100, is_repr=True)
    schedule: "Schedule | None" = declare_property(110)
