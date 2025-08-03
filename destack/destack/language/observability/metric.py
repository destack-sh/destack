from typing import TYPE_CHECKING

from destack.language.core import (
    Entity,
    Event,
    NodeType,
    builtin_entity,
    builtin_event,
    builtin_property,
)

if TYPE_CHECKING:
    from destack.language import Icon

# pyright: reportIncompatibleVariableOverride=false


@builtin_entity(NodeType.METRIC, is_abstract=True)
class Metric(Entity):
    """An Entity that represents a Metric."""

    icon: "Icon | None" = builtin_property(102)


@builtin_event(NodeType.MEASUREMENT_EVENT, is_abstract=True)
class MeasurementEvent(Event):
    """An Event that represents a Measurement."""

    definition: "Metric" = builtin_property(
        6,
        is_internal=True,
        is_readonly=True,
    )