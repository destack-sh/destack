from typing import TYPE_CHECKING

from destack.core import NodeType, builtin_entity, builtin_event, builtin_property

from .metric import MeasurementEvent, Metric

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@builtin_entity(
    NodeType.HISTOGRAM_METRIC,
    event_types=(NodeType.HISTOGRAM_MEASUREMENT_EVENT,),
)
class HistogramMetric(Metric):
    """A Histogram Metric."""

    pass


@builtin_event(NodeType.HISTOGRAM_MEASUREMENT_EVENT)
class HistogramMeasurementEvent(MeasurementEvent):
    """A Histogram Measurement."""

    definition: "HistogramMetric" = builtin_property(6)
