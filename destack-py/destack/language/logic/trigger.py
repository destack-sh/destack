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
from destack.proto import TriggerProto, TriggerStartedEventProto, TriggerStoppedEventProto
from destack.utils.uuid import UUID

if TYPE_CHECKING:
    from destack.language import Condition

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.TRIGGER_STARTED_EVENT)
class TriggerStartedEvent(
    Event["Trigger"],
    Node[TriggerStartedEventProto],
):
    """A Event regarding a Trigger."""

    node: "Trigger" = property_(35)


@builtin_node(NodeType.TRIGGER_STOPPED_EVENT)
class TriggerStoppedEvent(
    Event["Trigger"],
    Node[TriggerStoppedEventProto],
):
    """A Event regarding a Trigger."""

    node: "Trigger" = property_(35)


@builtin_enum(EnumType.TRIGGER_TYPE)
class TriggerType(Enum):
    EVENT = 1


@builtin_node(NodeType.TRIGGER)
class Trigger(
    Spatial,
    Entity,
    HasName,
    Node[TriggerProto],
):
    """A Trigger is a dynamic event to run something."""

    # when
    event: Optional[RelationReference] = property_(40)
    where: Optional["Condition"] = property_(41)
    # sampling?

    # what
    target: IsRunnable = property_(50)
    arguments: dict[UUID, Value] = property_(51)
