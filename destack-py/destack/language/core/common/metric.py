from destack.proto import (
    CounterMeasurementProto,
    CounterMetricProto,
    GaugeMeasurementProto,
    GaugeMetricProto,
    HistogramMeasurementProto,
    HistogramMetricProto,
)

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
    Node[GaugeMetricProto],
):
    """A Gauge Metric."""

    pass


@builtin_node(NodeType.GAUGE_MEASUREMENT)
class GaugeMeasurement(
    Spatial,
    Measurement,
    Node[GaugeMeasurementProto],
):
    """A Gauge Measurement."""

    definition: "GaugeMetric" = property_(6)


@builtin_node(NodeType.COUNTER_METRIC)
class CounterMetric(
    Spatial,
    Metric,
    HasName,
    Node[CounterMetricProto],
):
    """A Counter Metric."""

    pass


@builtin_node(NodeType.COUNTER_MEASUREMENT)
class CounterMeasurement(
    Spatial,
    Measurement,
    Node[CounterMeasurementProto],
):
    """A Counter Measurement."""

    definition: "CounterMetric" = property_(6)


@builtin_node(NodeType.HISTOGRAM_METRIC)
class HistogramMetric(
    Spatial,
    Metric,
    HasName,
    Node[HistogramMetricProto],
):
    """A Histogram Metric."""

    pass


@builtin_node(NodeType.HISTOGRAM_MEASUREMENT)
class HistogramMeasurement(
    Spatial,
    Measurement,
    Node[HistogramMeasurementProto],
):
    """A Histogram Measurement."""

    definition: "HistogramMetric" = property_(6)
