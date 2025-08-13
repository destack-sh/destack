from typing import TYPE_CHECKING

from destack.core import (
    Entity,
    Event,
    NodeType,
    ReferenceType,
    declare_entity,
    declare_event,
    declare_property,
)

if TYPE_CHECKING:
    pass


@declare_entity(NodeType.METRIC, is_abstract=True)
class Metric(Entity):
    """An Entity that represents a Metric."""


@declare_event(NodeType.MEASUREMENT_EVENT, is_abstract=True)
class MeasurementEvent(Event):
    """An Event that represents a Measurement."""

    definition: "Metric" = declare_property(
        10,
        is_managed=True,
        is_readonly=True,
        reference_type=ReferenceType.SPATIAL,
        tag=None,
    )
