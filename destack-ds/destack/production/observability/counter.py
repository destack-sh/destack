from typing import TYPE_CHECKING

from destack.core import NodeType, ReferenceType, declare_entity, declare_event, declare_property

from .metric import MeasurementEvent, Metric

if TYPE_CHECKING:
    pass


@declare_entity(
    NodeType.COUNTER_METRIC,
    event_types=(NodeType.COUNTER_MEASUREMENT_EVENT,),
)
class CounterMetric(Metric):
    """A Counter Metric."""

    pass


@declare_event(NodeType.COUNTER_MEASUREMENT_EVENT)
class CounterMeasurementEvent(MeasurementEvent):
    """A Counter Measurement."""

    definition: "CounterMetric" = declare_property(
        10,
        is_managed=True,
        is_readonly=True,
        reference_type=ReferenceType.SPATIAL,
        tag=None,
    )
