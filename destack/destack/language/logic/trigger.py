from typing import TYPE_CHECKING, Optional, Union

from destack.language.core import (
    Entity,
    Enum,
    EnumType,
    Event,
    NodeType,
    Value,
    builtin_entity,
    builtin_enum,
    builtin_event,
    builtin_property,
)
from destack.utils.uuid import UUID

if TYPE_CHECKING:
    from destack.language import Condition, Icon

# pyright: reportIncompatibleVariableOverride=false


@builtin_event(NodeType.TRIGGER_EVENT, is_abstract=True)
class TriggerEvent(Event):
    """A TriggerEvent is an Event that corresponds to a Trigger."""

    trigger: "Trigger" = builtin_property(101)


@builtin_enum(EnumType.TRIGGER_TYPE)
class TriggerType(Enum):
    EVENT = 1


@builtin_entity(NodeType.TRIGGER)
class Trigger(Entity):
    """A Trigger is a dynamic event to run something."""

    icon: "Icon | None" = builtin_property(102)

    # when
    event: Optional[NodeType] = builtin_property(110)
    where: Optional["Condition"] = builtin_property(111)
    # sampling?
    # is_passive/scope/process_mode/liveness?
    #  (only trigger if containing View? is active, no backfill)

    # what
    target: Union["Entity", None] = builtin_property(120)
    arguments: dict[UUID, Value] = builtin_property(121)
