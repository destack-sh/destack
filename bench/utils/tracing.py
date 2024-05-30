from pathlib import Path
from time import time_ns
from typing import Any, cast

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
from opentelemetry.sdk.trace import TracerProvider
from opentelemetry.sdk.trace.export import BatchSpanProcessor

from bench.utils.env import ENVIRONMENT, IS_DEV
from bench.utils.utils import get_from_env

VERSION = Path("version").read_text().strip()
OLTP_ENDPOINT = get_from_env("OTLP_ENDPOINT")

_setup_tracing = False
_processor: BatchSpanProcessor | None
_reader: PeriodicExportingMetricReader | None


def setup_tracing():
    global _processor
    global _reader
    global _setup_tracing
    if _setup_tracing:
        return

    resource = Resource(
        attributes={
            SERVICE_NAME: get_from_env("SERVICE_NAME", default="cli" if IS_DEV else None),
            SERVICE_VERSION: VERSION,
            DEPLOYMENT_ENVIRONMENT: ENVIRONMENT,
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


setup_tracing()
