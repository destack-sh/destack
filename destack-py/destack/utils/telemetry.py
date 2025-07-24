import sys
from collections.abc import Mapping
from pathlib import Path
from time import time_ns
from typing import Any, Optional, cast

import structlog
from opentelemetry import baggage, context, metrics, trace
from opentelemetry.baggage.propagation import W3CBaggagePropagator
from opentelemetry.sdk.metrics import MeterProvider
from opentelemetry.sdk.metrics.export import PeriodicExportingMetricReader
from opentelemetry.sdk.resources import (
    DEPLOYMENT_ENVIRONMENT,
    SERVICE_INSTANCE_ID,
    SERVICE_NAME,
    SERVICE_VERSION,
    Resource,
)
from opentelemetry.sdk.trace import Span, TracerProvider
from opentelemetry.sdk.trace.export import BatchSpanProcessor
from opentelemetry.trace.propagation.tracecontext import TraceContextTextMapPropagator

from destack.utils.uuid import UUID, uuid4

from .env import ENV, IS_DEV, IS_TEST, get_from_env, get_from_env_maybe
from .log import setup_logging

setup_logging()  # ensure logging is setup first
logger = structlog.get_logger(__name__)

IS_DEBUG: bool = hasattr(sys, "gettrace") and sys.gettrace() is not None
VERSION = Path("version").read_text().strip()
OTLP_ENDPOINT = get_from_env_maybe("OTLP_ENDPOINT", description="Full URL to send OTLP traces to")
TELEMETRY = not (IS_DEV or IS_TEST)

_did_setup_telemetry = False
_processor: BatchSpanProcessor | None = None
_reader: PeriodicExportingMetricReader | None = None


class BaggageBatchSpanProcessor(BatchSpanProcessor):
    def on_start(self, span: Span, parent_context: Optional[Any] = None) -> None:
        super().on_start(span, parent_context)
        # inherit baggage as span attributes
        for key, value in baggage.get_all(parent_context).items():
            span.set_attribute(key, value)  # type: ignore


def _render_value(value: Any):
    if isinstance(value, (str, int, float, bool)):
        return value
    elif isinstance(value, UUID):
        return str(value)
    else:
        return repr(value)


def set_baggage(**kwargs):
    for key, value in kwargs.items():
        if value is None:
            continue
        key = key.replace("__", ".")
        context.attach(baggage.set_baggage(key, _render_value(value)))


def set_user(user: UUID | None = None, name: str | None = None):
    """Sets the user for telemetry."""
    user_id = str(user) if user else None
    baggage.set_baggage("user.id", user_id)
    baggage.set_baggage("user.name", name)


def set_space(space: UUID | None = None, name: str | None = None, client: UUID | None = None):
    """Sets the space for telemetry."""
    space_id = str(space) if space else None
    client_id = str(client) if client else None
    baggage.set_baggage("space.id", space_id)
    baggage.set_baggage("space.name", name)
    baggage.set_baggage("client.id", client_id)


def handle_exception(exception: Exception) -> None:
    """Calls telemetry providers to capture exception (uses span)."""
    span = trace.get_current_span()
    span.record_exception(exception)
    span.set_status(trace.Status(trace.StatusCode.ERROR, str(exception)))


def setup_telemetry() -> None:
    """Sets up telemetry providers."""
    global _processor, _reader, _did_setup_telemetry
    if _did_setup_telemetry:
        return
    if not TELEMETRY:
        logger.debug("telemetry.skip")
        _did_setup_telemetry = True
        return

    logger.debug("telemetry.setup")

    # otlp
    if OTLP_ENDPOINT:
        from opentelemetry.exporter.otlp.proto.grpc.metric_exporter import OTLPMetricExporter
        from opentelemetry.exporter.otlp.proto.grpc.trace_exporter import OTLPSpanExporter

        resource = Resource(
            attributes={
                SERVICE_NAME: get_from_env(
                    "SERVICE_NAME", default="cli" if IS_DEV else None, description="Service name"
                ),
                SERVICE_INSTANCE_ID: str(uuid4()),
                SERVICE_VERSION: VERSION,
                DEPLOYMENT_ENVIRONMENT: ENV,
            }
        )
        tracer_provider = TracerProvider(resource=resource)
        _processor = BaggageBatchSpanProcessor(
            OTLPSpanExporter(endpoint=OTLP_ENDPOINT, insecure=True)
        )
        tracer_provider.add_span_processor(_processor)
        trace.set_tracer_provider(tracer_provider)

        _reader = PeriodicExportingMetricReader(
            OTLPMetricExporter(endpoint=OTLP_ENDPOINT, insecure=True),
        )
        meter_provider = MeterProvider(resource=resource, metric_readers=[_reader])
        metrics.set_meter_provider(meter_provider)
        logger.debug("telemetry.otlp.setup", endpoint=OTLP_ENDPOINT)
    else:
        logger.debug("telemetry.oltp.disabled")

    _did_setup_telemetry = True


def collect_propagation_context():
    """Compile current baggage and trace context into a propagation context."""
    headers = {}
    W3CBaggagePropagator().inject(headers)
    TraceContextTextMapPropagator().inject(headers)
    return headers


def attach_propagation_context(headers: Mapping[Any, Any]):
    """Attaches external baggage and trace context from a propagation context."""
    ctx = W3CBaggagePropagator().extract(headers)
    ctx = TraceContextTextMapPropagator().extract(headers, ctx)
    context.attach(ctx)


def export_now():
    """Exports all pending spans and metrics."""
    global _processor, _reader
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
            if self._start_time is not None:  # type: ignore
                return  # ignore double start
            self._start_time = start_time if start_time is not None else time_ns()

        self._span_processor.on_start(self, parent_context=parent_context)

    def end(self, end_time: Optional[int] = None) -> None:
        with self._lock:
            if self._start_time is None:  # type: ignore
                raise RuntimeError("Calling end() on a not started span.")
            if self._end_time is not None:  # type: ignore
                return  # ignore double end

            self._end_time = end_time if end_time is not None else time_ns()

        self._span_processor.on_end(self._readable_span())


Span.start = _PatchedSpan.start  # type: ignore
Span.end = _PatchedSpan.end  # type: ignore


setup_telemetry()
