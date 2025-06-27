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
from destack.proto import TimerProto, TimerStartedEventProto, TimerStoppedEventProto

from .schedule import Schedule

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.TIMER_STARTED_EVENT)
class TimerStartedEvent(
    Event["Timer"],
    Node[TimerStartedEventProto],
):
    """A Event regarding a Timer."""

    node: "Timer" = property_(35)


@builtin_node(NodeType.TIMER_STOPPED_EVENT)
class TimerStoppedEvent(
    Event["Timer"],
    Node[TimerStoppedEventProto],
):
    """A Event regarding a Timer."""

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
