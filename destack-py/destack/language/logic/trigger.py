from typing import TYPE_CHECKING, Optional, Union

from destack.language.core import (
    Entity,
    Enum,
    EnumType,
    Event,
    IsSpatial,
    NodeDefinitionReference,
    NodeType,
    Value,
    builtin_enum,
    builtin_node,
    builtin_property,
)
from destack.utils.uuid import UUID

if TYPE_CHECKING:
    from destack.language import Action, Condition, Icon, Service

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.TRIGGER_EVENT, frozen=True, is_abstract=True)
class TriggerEvent(Event["Trigger"]):
    """A TriggerEvent is an Event that corresponds to a Trigger."""

    node: "Trigger" = builtin_property(101)


@builtin_enum(EnumType.TRIGGER_TYPE)
class TriggerType(Enum):
    EVENT = 1


@builtin_node(
    NodeType.TRIGGER,
)
class Trigger(IsSpatial, Entity):
    """A Trigger is a dynamic event to run something."""

    name: str = builtin_property(101, is_repr=True)
    icon: "Icon | None" = builtin_property(102)

    # when
    event: Optional[NodeDefinitionReference] = builtin_property(110)
    where: Optional["Condition"] = builtin_property(111)
    # sampling?
    # is_passive? (only trigger if containing View? is active, no backfill)

    # what
    target: Union["Action", "Service", None] = builtin_property(120)
    arguments: dict[UUID, Value] = builtin_property(121)
