from typing import TYPE_CHECKING

from destack.core import NodeType, ReferenceType, declare_entity, declare_event, declare_property

from .metric import MeasurementEvent, Metric

if TYPE_CHECKING:
    pass


@declare_entity(
    NodeType.GAUGE_METRIC,
    event_types=(NodeType.GAUGE_MEASUREMENT_EVENT,),
)
class GaugeMetric(Metric):
    """A Gauge Metric."""

    pass


@declare_event(NodeType.GAUGE_MEASUREMENT_EVENT)
class GaugeMeasurementEvent(MeasurementEvent):
    """A Gauge Measurement."""

    definition: "GaugeMetric" = declare_property(
        10,
        is_managed=True,
        is_readonly=True,
        reference_type=ReferenceType.SPATIAL,
        tag=None,
    )
