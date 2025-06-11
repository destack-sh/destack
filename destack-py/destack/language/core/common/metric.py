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
    IsSourceable,
    Measurement,
    Metric,
    Node,
    NodeType,
    Spatial,
    node_,
    property_,
)

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.GAUGE_METRIC)
class GaugeMetric(
    Spatial,
    Metric,
    HasName,
    IsSourceable,
    Node[GaugeMetricData],
):
    """A Gauge Metric."""

    pass


@node_(NodeType.GAUGE_MEASUREMENT)
class GaugeMeasurement(
    Spatial,
    Measurement,
    Node[GaugeMeasurementData],
):
    """A Gauge Measurement."""

    definition: "GaugeMetric" = property_(17)


@node_(NodeType.COUNTER_METRIC)
class CounterMetric(
    Spatial,
    Metric,
    HasName,
    IsSourceable,
    Node[CounterMetricData],
):
    """A Counter Metric."""

    pass


@node_(NodeType.COUNTER_MEASUREMENT)
class CounterMeasurement(
    Spatial,
    Measurement,
    Node[CounterMeasurementData],
):
    """A Counter Measurement."""

    definition: "CounterMetric" = property_(17)


@node_(NodeType.HISTOGRAM_METRIC)
class HistogramMetric(
    Spatial,
    Metric,
    HasName,
    IsSourceable,
    Node[HistogramMetricData],
):
    """A Histogram Metric."""

    pass


@node_(NodeType.HISTOGRAM_MEASUREMENT)
class HistogramMeasurement(
    Spatial,
    Measurement,
    Node[HistogramMeasurementData],
):
    """A Histogram Measurement."""

    definition: "HistogramMetric" = property_(17)
