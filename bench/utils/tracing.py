from pathlib import Path
from time import time_ns
from typing import Any, Optional, cast

from opentelemetry import metrics, trace
from opentelemetry.exporter.otlp.proto.grpc.metric_exporter import OTLPMetricExporter
from opentelemetry.exporter.otlp.proto.grpc.trace_exporter import OTLPSpanExporter
from opentelemetry.sdk.metrics import MeterProvider
from opentelemetry.sdk.metrics.export import PeriodicExportingMetricReader
from opentelemetry.sdk.resources import (
    DEPLOYMENT_ENVIRONMENT,
    SERVICE_NAME,
    SERVICE_VERSION,
    Resource,
)
from opentelemetry.sdk.trace import Span, TracerProvider
from opentelemetry.sdk.trace.export import BatchSpanProcessor

from bench.utils.env import EMV, IS_DEBUG, IS_DEV, IS_TEST
from bench.utils.utils import get_from_env

VERSION = Path("version").read_text().strip()

_setup_tracing = False
_processor: BatchSpanProcessor | None
_reader: PeriodicExportingMetricReader | None


def setup_tracing():
    global _processor
    global _reader
    global _setup_tracing
    if _setup_tracing:
        return
    if IS_DEBUG or IS_TEST:
        # NOTE: we don't trace in debug mode because it's pretty slow
        return

    OLTP_ENDPOINT = get_from_env("OTLP_ENDPOINT")
    resource = Resource(
        attributes={
            SERVICE_NAME: get_from_env("SERVICE_NAME", default="cli" if IS_DEV else None),
            SERVICE_VERSION: VERSION,
            DEPLOYMENT_ENVIRONMENT: EMV,
        }
    )
    tracer_provider = TracerProvider(resource=resource)
    _processor = BatchSpanProcessor(OTLPSpanExporter(endpoint=OLTP_ENDPOINT, insecure=True))
    tracer_provider.add_span_processor(_processor)
    trace.set_tracer_provider(tracer_provider)

    reader = PeriodicExportingMetricReader(
        OTLPMetricExporter(endpoint=OLTP_ENDPOINT, insecure=True),
    )
    meter_provider = MeterProvider(resource=resource, metric_readers=[reader])
    metrics.set_meter_provider(meter_provider)

    _setup_tracing = True


def export_now():
    """Exports all pending spans and metrics."""
    if _processor:
        _processor.force_flush()
    if _reader:
        _reader.force_flush()


def get_span_duration(span: trace.Span):
    """Gets the duration of the span in seconds (to now if unfinished)"""
    return (getattr(span, "end_time") or time_ns() - cast(Any, span).start_time) / 1_000_000_000


class _PatchedSpan(Span):
    """Monkey-patch opentelemetry Span to ignore double start/end calls (fine for us)."""

    def start(
        self,
        start_time: Optional[int] = None,
        parent_context: Optional[Any] = None,
    ) -> None:
        with self._lock:
            if self._start_time is not None:
                return  # ignore double start
            self._start_time = start_time if start_time is not None else time_ns()

        self._span_processor.on_start(self, parent_context=parent_context)

    def end(self, end_time: Optional[int] = None) -> None:
        with self._lock:
            if self._start_time is None:
                raise RuntimeError("Calling end() on a not started span.")
            if self._end_time is not None:
                return  # ignore double end

            self._end_time = end_time if end_time is not None else time_ns()

        self._span_processor.on_end(self._readable_span())


Span.start = _PatchedSpan.start  # type: ignore
Span.end = _PatchedSpan.end  # type: ignore


setup_tracing()
