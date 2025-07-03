from ..builtin import (
    MeasurementEvent,
    Metric,
    NodeType,
    builtin_node,
    builtin_property,
)

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.GAUGE_METRIC)
class GaugeMetric(Metric):
    """A Gauge Metric."""

    pass


@builtin_node(NodeType.GAUGE_MEASUREMENT_EVENT, frozen=True)
class GaugeMeasurementEvent(MeasurementEvent):
    """A Gauge Measurement."""

    definition: "GaugeMetric" = builtin_property(6)


@builtin_node(NodeType.COUNTER_METRIC)
class CounterMetric(Metric):
    """A Counter Metric."""

    pass


@builtin_node(NodeType.COUNTER_MEASUREMENT_EVENT, frozen=True)
class CounterMeasurementEvent(MeasurementEvent):
    """A Counter Measurement."""

    definition: "CounterMetric" = builtin_property(6)


@builtin_node(NodeType.HISTOGRAM_METRIC)
class HistogramMetric(Metric):
    """A Histogram Metric."""

    pass


@builtin_node(NodeType.HISTOGRAM_MEASUREMENT_EVENT, frozen=True)
class HistogramMeasurementEvent(MeasurementEvent):
    """A Histogram Measurement."""

    definition: "HistogramMetric" = builtin_property(6)
