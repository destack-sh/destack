from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Entity,
    Enum,
    EnumType,
    Event,
    HasName,
    IsRunnable,
    Node,
    NodeType,
    RelationReference,
    Spatial,
    Value,
    builtin_enum,
    builtin_node,
    property_,
)
from destack.pb2 import TriggerData, TriggerEventData
from destack.utils.uuid import UUID

if TYPE_CHECKING:
    from destack.language import Condition

# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.TRIGGER_EVENT_TYPE)
class TriggerEventType(Enum):
    """A Type of Trigger Event."""

    STARTED = 1, "Started", "Started", "fas fa-play"
    TRIGGERED = 2, "Triggered", "Triggered", "fas fa-play"
    STOPPED = 3, "Stopped", "Stopped", "fas fa-stop"


@builtin_node(NodeType.TRIGGER_EVENT)
class TriggerEvent(
    Event["Trigger"],
    Node[TriggerEventData],
):
    """A Event regarding a Trigger."""

    type: TriggerEventType = property_(30)
    node: "Trigger" = property_(35)


@builtin_enum(EnumType.TRIGGER_TYPE)
class TriggerType(Enum):
    EVENT = 1


@builtin_node(NodeType.TRIGGER)
class Trigger(
    Spatial,
    Entity,
    HasName,
    Node[TriggerData],
):
    """A Trigger is a dynamic event to run something."""

    type: TriggerType = property_(30)

    # when
    event: Optional[RelationReference] = property_(40)
    where: Optional["Condition"] = property_(41)
    # sampling?

    # what
    target: IsRunnable = property_(50)
    arguments: dict[UUID, Value] = property_(51)
