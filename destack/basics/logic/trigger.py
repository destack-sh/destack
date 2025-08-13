from typing import TYPE_CHECKING, Optional

from destack.core import (
    UUID,
    Entity,
    EnumType,
    Event,
    NodeType,
    OptionEnum,
    ReferenceType,
    Value,
    declare_entity,
    declare_enum,
    declare_event,
    declare_option,
    declare_property,
)

if TYPE_CHECKING:
    from destack import Condition


@declare_event(NodeType.TRIGGER_EVENT, is_abstract=True)
class TriggerEvent(Event):
    """A TriggerEvent is an Event that corresponds to a Trigger."""

    trigger: "Trigger" = declare_property(
        101,
        reference_type=ReferenceType.SPATIAL,
        tag=None,
    )


@declare_enum(EnumType.TRIGGER_TYPE)
class TriggerType(OptionEnum):
    EVENT = declare_option(1, "Event", description="A Trigger that runs on an Event")


@declare_entity(NodeType.TRIGGER)
class Trigger(Entity):
    """A Trigger is a dynamic event to run something."""

    # when
    event: Optional[NodeType] = declare_property(110, tag=None)
    where: Optional["Condition"] = declare_property(111, tag=None)
    # sampling?
    # is_passive/scope/process_mode/liveness?
    #  (only trigger if containing View? is active, no backfill)

    # what
    target: Optional["Entity"] = declare_property(
        120,
        reference_type=ReferenceType.SPATIAL,
        tag=None,
    )
    arguments: dict[UUID, Value] = declare_property(121, tag=None)
