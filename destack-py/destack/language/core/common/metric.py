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
    IsMeasurement,
    IsMetric,
    IsSourceable,
    IsSpatial,
    Node,
    NodeType,
    node_,
    property_,
)

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.GAUGE_METRIC)
class GaugeMetric(
    HasName,
    IsMetric,
    IsSourceable,
    IsSpatial,
    Node[GaugeMetricData],
):
    """A Gauge Metric."""

    pass


@node_(NodeType.GAUGE_MEASUREMENT)
class GaugeMeasurement(
    IsMeasurement,
    Node[GaugeMeasurementData],
):
    """A Gauge Measurement."""

    definition: "GaugeMetric" = property_(17)


@node_(NodeType.COUNTER_METRIC)
class CounterMetric(
    HasName,
    IsMetric,
    IsSourceable,
    IsSpatial,
    Node[CounterMetricData],
):
    """A Counter Metric."""

    pass


@node_(NodeType.COUNTER_MEASUREMENT)
class CounterMeasurement(
    IsMeasurement,
    Node[CounterMeasurementData],
):
    """A Counter Measurement."""

    definition: "CounterMetric" = property_(17)


@node_(NodeType.HISTOGRAM_METRIC)
class HistogramMetric(
    HasName,
    IsMetric,
    IsSourceable,
    IsSpatial,
    Node[HistogramMetricData],
):
    """A Histogram Metric."""

    pass


@node_(NodeType.HISTOGRAM_MEASUREMENT)
class HistogramMeasurement(
    IsMeasurement,
    Node[HistogramMeasurementData],
):
    """A Histogram Measurement."""

    definition: "HistogramMetric" = property_(17)
