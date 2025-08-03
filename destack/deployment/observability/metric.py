from typing import TYPE_CHECKING

from destack.core import (
    Entity,
    Event,
    NodeType,
    declare_entity,
    declare_event,
    declare_property,
)

if TYPE_CHECKING:
    from destack import Icon

# pyright: reportIncompatibleVariableOverride=false


@declare_entity(NodeType.METRIC, is_abstract=True)
class Metric(Entity):
    """An Entity that represents a Metric."""

    icon: "Icon | None" = declare_property(102)


@declare_event(NodeType.MEASUREMENT_EVENT, is_abstract=True)
class MeasurementEvent(Event):
    """An Event that represents a Measurement."""

    definition: "Metric" = declare_property(
        6,
        is_internal=True,
        is_readonly=True,
    )
