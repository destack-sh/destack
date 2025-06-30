from ..builtin import (
    MeasurementEvent,
    Metric,
    NodeType,
    builtin_node,
    property_,
)

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.GAUGE_METRIC)
class GaugeMetric(Metric):
    """A Gauge Metric."""

    pass


@builtin_node(NodeType.GAUGE_MEASUREMENT_EVENT)
class GaugeMeasurementEvent(MeasurementEvent):
    """A Gauge Measurement."""

    definition: "GaugeMetric" = property_(6)


@builtin_node(NodeType.COUNTER_METRIC)
class CounterMetric(Metric):
    """A Counter Metric."""

    pass


@builtin_node(NodeType.COUNTER_MEASUREMENT_EVENT)
class CounterMeasurementEvent(MeasurementEvent):
    """A Counter Measurement."""

    definition: "CounterMetric" = property_(6)


@builtin_node(NodeType.HISTOGRAM_METRIC)
class HistogramMetric(Metric):
    """A Histogram Metric."""

    pass


@builtin_node(NodeType.HISTOGRAM_MEASUREMENT_EVENT)
class HistogramMeasurementEvent(MeasurementEvent):
    """A Histogram Measurement."""

    definition: "HistogramMetric" = property_(6)
