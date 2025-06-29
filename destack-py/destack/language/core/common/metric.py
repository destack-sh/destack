from ..builtin import (
    HasName,
    Measurement,
    Metric,
    Node,
    NodeType,
    Spatial,
    builtin_node,
    property_,
)

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.GAUGE_METRIC)
class GaugeMetric(
    Spatial,
    Metric,
    HasName,
    Node,
):
    """A Gauge Metric."""

    pass


@builtin_node(NodeType.GAUGE_MEASUREMENT)
class GaugeMeasurement(
    Measurement,
    Node,
):
    """A Gauge Measurement."""

    definition: "GaugeMetric" = property_(6)


@builtin_node(NodeType.COUNTER_METRIC)
class CounterMetric(
    Spatial,
    Metric,
    HasName,
    Node,
):
    """A Counter Metric."""

    pass


@builtin_node(NodeType.COUNTER_MEASUREMENT)
class CounterMeasurement(
    Measurement,
    Node,
):
    """A Counter Measurement."""

    definition: "CounterMetric" = property_(6)


@builtin_node(NodeType.HISTOGRAM_METRIC)
class HistogramMetric(
    Spatial,
    Metric,
    HasName,
    Node,
):
    """A Histogram Metric."""

    pass


@builtin_node(NodeType.HISTOGRAM_MEASUREMENT)
class HistogramMeasurement(
    Measurement,
    Node,
):
    """A Histogram Measurement."""

    definition: "HistogramMetric" = property_(6)
