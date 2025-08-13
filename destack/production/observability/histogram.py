from typing import TYPE_CHECKING

from destack.core import NodeType, ReferenceType, declare_entity, declare_event, declare_property

from .metric import MeasurementEvent, Metric

if TYPE_CHECKING:
    pass


@declare_entity(
    NodeType.HISTOGRAM_METRIC,
    event_types=(NodeType.HISTOGRAM_MEASUREMENT_EVENT,),
)
class HistogramMetric(Metric):
    """A Histogram Metric."""

    pass


@declare_event(NodeType.HISTOGRAM_MEASUREMENT_EVENT)
class HistogramMeasurementEvent(MeasurementEvent):
    """A Histogram Measurement."""

    definition: "HistogramMetric" = declare_property(
        10,
        is_managed=True,
        is_readonly=True,
        reference_type=ReferenceType.SPATIAL,
        tag=None,
    )
