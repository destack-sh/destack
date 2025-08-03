from typing import TYPE_CHECKING

from destack.core import NodeType, builtin_entity, builtin_event, builtin_property

from .metric import MeasurementEvent, Metric

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@builtin_entity(
    NodeType.COUNTER_METRIC,
    event_types=(NodeType.COUNTER_MEASUREMENT_EVENT,),
)
class CounterMetric(Metric):
    """A Counter Metric."""

    pass


@builtin_event(NodeType.COUNTER_MEASUREMENT_EVENT)
class CounterMeasurementEvent(MeasurementEvent):
    """A Counter Measurement."""

    definition: "CounterMetric" = builtin_property(6)
