from typing import TYPE_CHECKING

from fastuuid import UUID

from destack.language.core import (
    BuiltinEnum,
    EnumType,
    IsEntity,
    Node,
    NodeType,
    Value,
    enum_,
    node_,
    property_,
)
from destack.language.core.builtin.trait import IsRunnable

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.TRIGGER_TYPE)
class TriggerType(BuiltinEnum):
    EVENT = 1


@node_(NodeType.TRIGGER)
class Trigger(IsEntity, Node):
    """A Trigger is a dynamic event to run something."""

    type: TriggerType = property_(30)

    target: IsRunnable = property_(40)
    arguments: dict[UUID, Value] = property_(41)
