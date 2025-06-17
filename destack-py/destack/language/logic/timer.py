from typing import TYPE_CHECKING

from destack.language.core import (
    Entity,
    Enum,
    EnumType,
    Event,
    HasName,
    Node,
    NodeType,
    Spatial,
    builtin_enum,
    builtin_node,
    property_,
)
from destack.proto import TimerEventProto, TimerProto

from .schedule import Schedule

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.TIMER_EVENT_TYPE)
class TimerEventType(Enum):
    """A Type of Timer Event."""

    STARTED = 1, "Started", "Started", "fas fa-play"
    STOPPED = 2, "Stopped", "Stopped", "fas fa-stop"
    EXPIRED = 3, "Expired", "Expired", "fas fa-clock"


@builtin_node(NodeType.TIMER_EVENT)
class TimerEvent(
    Event["Timer"],
    Node[TimerEventProto],
):
    """A Event regarding a Timer."""

    type: TimerEventType = property_(30)
    node: "Timer" = property_(35)


@builtin_enum(EnumType.TIMER_TYPE)
class TimerType(Enum):
    ONCE = 1
    RECURRING = 2


@builtin_node(NodeType.TIMER)
class Timer(
    Spatial,
    Entity,
    HasName,
    Node[TimerProto],
):
    """A Timer."""

    type: TimerType = property_(30)
    schedule: "Schedule | None" = property_(40)
