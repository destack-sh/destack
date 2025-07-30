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
