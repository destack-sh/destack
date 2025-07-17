from typing import TYPE_CHECKING

from destack.language.core import (
    Entity,
    Event,
    IsSourceable,
    NodeType,
    builtin_node,
    builtin_property,
)

if TYPE_CHECKING:
    from destack.language import Icon

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.METRIC, is_abstract=True)
class Metric(IsSourceable, Entity):
    """An Entity that represents a Metric."""

    icon: "Icon | None" = builtin_property(102)


@builtin_node(NodeType.MEASUREMENT_EVENT, frozen=True, is_abstract=True)
class MeasurementEvent(Event):
    """An Event that represents a Measurement."""

    definition: "Metric" = builtin_property(
        6,
        is_managed=True,
        is_readonly=True,
    )


@builtin_node(
    NodeType.GAUGE_METRIC,
    event_types=(NodeType.GAUGE_MEASUREMENT_EVENT,),
)
class GaugeMetric(Metric):
    """A Gauge Metric."""

    pass


@builtin_node(NodeType.GAUGE_MEASUREMENT_EVENT, frozen=True)
class GaugeMeasurementEvent(MeasurementEvent):
    """A Gauge Measurement."""

    definition: "GaugeMetric" = builtin_property(6)


@builtin_node(
    NodeType.COUNTER_METRIC,
    event_types=(NodeType.COUNTER_MEASUREMENT_EVENT,),
)
class CounterMetric(Metric):
    """A Counter Metric."""

    pass


@builtin_node(NodeType.COUNTER_MEASUREMENT_EVENT, frozen=True)
class CounterMeasurementEvent(MeasurementEvent):
    """A Counter Measurement."""

    definition: "CounterMetric" = builtin_property(6)


@builtin_node(
    NodeType.HISTOGRAM_METRIC,
    event_types=(NodeType.HISTOGRAM_MEASUREMENT_EVENT,),
)
class HistogramMetric(Metric):
    """A Histogram Metric."""

    pass


@builtin_node(NodeType.HISTOGRAM_MEASUREMENT_EVENT, frozen=True)
class HistogramMeasurementEvent(MeasurementEvent):
    """A Histogram Measurement."""

    definition: "HistogramMetric" = builtin_property(6)
