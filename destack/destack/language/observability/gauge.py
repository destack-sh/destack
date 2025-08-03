from typing import TYPE_CHECKING

from destack.language.core import NodeType, builtin_entity, builtin_event, builtin_property

from .metric import MeasurementEvent, Metric

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@builtin_entity(
    NodeType.GAUGE_METRIC,
    event_types=(NodeType.GAUGE_MEASUREMENT_EVENT,),
)
class GaugeMetric(Metric):
    """A Gauge Metric."""

    pass


@builtin_event(NodeType.GAUGE_MEASUREMENT_EVENT)
class GaugeMeasurementEvent(MeasurementEvent):
    """A Gauge Measurement."""

    definition: "GaugeMetric" = builtin_property(6)