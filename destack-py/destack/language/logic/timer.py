from typing import TYPE_CHECKING

from destack.language.core import (
    BuiltinEnum,
    Entity,
    EnumType,
    Event,
    HasName,
    Node,
    NodeType,
    Spatial,
    enum_,
    node_,
    property_,
)
from destack.pb2 import TimerData

from .schedule import Schedule

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.TIMER_EVENT_TYPE)
class TimerEventType(BuiltinEnum):
    """A Type of Timer Event."""

    STARTED = 1, "Started", "Started", "fas fa-play"
    STOPPED = 2, "Stopped", "Stopped", "fas fa-stop"
    EXPIRED = 3, "Expired", "Expired", "fas fa-clock"


@node_(NodeType.TIMER_EVENT)
class TimerEvent(
    Spatial,
    Event["Timer"],
    Node["TimerEventData"],
):
    """A Event regarding a Timer."""

    type: TimerEventType = property_(30)
    node: "Timer" = property_(35)


@enum_(EnumType.TIMER_TYPE)
class TimerType(BuiltinEnum):
    ONCE = 1
    RECURRING = 2


@node_(NodeType.TIMER)
class Timer(
    Spatial,
    Entity,
    HasName,
    Node[TimerData],
):
    """A Timer."""

    type: TimerType = property_(30)
    schedule: "Schedule | None" = property_(40)
