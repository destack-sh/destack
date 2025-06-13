from destack.pb2 import (
    CounterMeasurementData,
    CounterMetricData,
    GaugeMeasurementData,
    GaugeMetricData,
    HistogramMeasurementData,
    HistogramMetricData,
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
    Node[GaugeMetricData],
):
    """A Gauge Metric."""

    pass


@builtin_node(NodeType.GAUGE_MEASUREMENT)
class GaugeMeasurement(
    Spatial,
    Measurement,
    Node[GaugeMeasurementData],
):
    """A Gauge Measurement."""

    definition: "GaugeMetric" = property_(17)


@builtin_node(NodeType.COUNTER_METRIC)
class CounterMetric(
    Spatial,
    Metric,
    HasName,
    Node[CounterMetricData],
):
    """A Counter Metric."""

    pass


@builtin_node(NodeType.COUNTER_MEASUREMENT)
class CounterMeasurement(
    Spatial,
    Measurement,
    Node[CounterMeasurementData],
):
    """A Counter Measurement."""

    definition: "CounterMetric" = property_(17)


@builtin_node(NodeType.HISTOGRAM_METRIC)
class HistogramMetric(
    Spatial,
    Metric,
    HasName,
    Node[HistogramMetricData],
):
    """A Histogram Metric."""

    pass


@builtin_node(NodeType.HISTOGRAM_MEASUREMENT)
class HistogramMeasurement(
    Spatial,
    Measurement,
    Node[HistogramMeasurementData],
):
    """A Histogram Measurement."""

    definition: "HistogramMetric" = property_(17)
