from typing import TYPE_CHECKING, Optional

from fastuuid import UUID

from destack.language.core import (
    BuiltinEnum,
    EnumType,
    HasName,
    IsEntity,
    IsRunnable,
    Node,
    NodeType,
    RelationReference,
    Value,
    enum_,
    node_,
    property_,
)

if TYPE_CHECKING:
    from destack.language import Condition

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.TRIGGER_TYPE)
class TriggerType(BuiltinEnum):
    EVENT = 1


@node_(NodeType.TRIGGER)
class Trigger(HasName, IsEntity, Node):
    """A Trigger is a dynamic event to run something."""

    type: TriggerType = property_(30)

    # when
    event: Optional[RelationReference] = property_(40)
    where: Optional["Condition"] = property_(41)

    # what
    target: IsRunnable = property_(50)
    arguments: dict[UUID, Value] = property_(51)
