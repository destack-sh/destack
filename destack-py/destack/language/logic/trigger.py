from typing import TYPE_CHECKING, Optional

from fastuuid import UUID

from destack.language.core import (
    BuiltinEnum,
    Entity,
    EnumType,
    Event,
    HasName,
    IsRunnable,
    Node,
    NodeType,
    RelationReference,
    Spatial,
    Value,
    enum_,
    node_,
    property_,
)
from destack.pb2 import TriggerData

if TYPE_CHECKING:
    from destack.language import Condition

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.TRIGGER_EVENT_TYPE)
class TriggerEventType(BuiltinEnum):
    """A Type of Trigger Event."""

    STARTED = 1, "Started", "Started", "fas fa-play"
    TRIGGERED = 2, "Triggered", "Triggered", "fas fa-play"
    STOPPED = 3, "Stopped", "Stopped", "fas fa-stop"


@node_(NodeType.TRIGGER_EVENT)
class TriggerEvent(
    Spatial,
    Event["Trigger"],
    Node["TriggerEventData"],
):
    """A Event regarding a Trigger."""

    type: TriggerEventType = property_(30)
    node: "Trigger" = property_(35)


@enum_(EnumType.TRIGGER_TYPE)
class TriggerType(BuiltinEnum):
    EVENT = 1


@node_(NodeType.TRIGGER)
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
