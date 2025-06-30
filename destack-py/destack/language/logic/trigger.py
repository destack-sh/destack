from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Entity,
    Enum,
    EnumType,
    Event,
    HasName,
    IsRunnable,
    IsSpatial,
    NodeDefinitionReference,
    NodeType,
    Value,
    builtin_enum,
    builtin_node,
    property_,
)
from destack.utils.uuid import UUID

if TYPE_CHECKING:
    from destack.language import Condition

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.TRIGGER_EVENT, is_abstract=True)
class TriggerEvent(Event["Trigger"]):
    """A TriggerEvent is an Event that corresponds to a Trigger."""

    node: "Trigger" = property_(35)


@builtin_enum(EnumType.TRIGGER_TYPE)
class TriggerType(Enum):
    EVENT = 1


@builtin_node(NodeType.TRIGGER)
class Trigger(IsSpatial, HasName, Entity):
    """A Trigger is a dynamic event to run something."""

    # when
    event: Optional[NodeDefinitionReference] = property_(40)
    where: Optional["Condition"] = property_(41)
    # sampling?

    # what
    target: IsRunnable = property_(50)
    arguments: dict[UUID, Value] = property_(51)
