import sys
from contextlib import contextmanager
from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from opentelemetry import trace

try:
    from opentelemetry import metrics, trace
    from opentelemetry.sdk.metrics import MeterProvider
    from opentelemetry.sdk.metrics.export import PeriodicExportingMetricReader
    from opentelemetry.sdk.resources import (
        DEPLOYMENT_ENVIRONMENT,
        SERVICE_INSTANCE_ID,
        SERVICE_NAME,
        SERVICE_VERSION,
        Resource,
    )
    from opentelemetry.sdk.trace import TracerProvider
    from opentelemetry.sdk.trace.export import BatchSpanProcessor

    from destack.utils.uuid import uuid4

    from .env import ENV, IS_DEV, IS_TEST, get_from_env, get_from_env_maybe
    from .log import get_logger, setup_logging

    setup_logging()  # ensure logging is setup first

    logger = get_logger(__name__)

    IS_DEBUG: bool = hasattr(sys, "gettrace") and sys.gettrace() is not None
    # look for version file in parent directory
    version_file = Path(__file__).parent.parent.parent.parent / "version"
    VERSION = version_file.read_text().strip() if version_file.exists() else "unknown"
    OTLP_ENDPOINT = get_from_env_maybe(
        "OTLP_ENDPOINT", description="Full URL to send OTLP traces to"
    )
    TELEMETRY = not (IS_DEV or IS_TEST)

    _setup_telemetry = False

    def setup_telemetry() -> None:
        """Sets up telemetry providers."""
        global _setup_telemetry
        if _setup_telemetry:
            return
        if not TELEMETRY:
            logger.debug("telemetry.skip")
            _setup_telemetry = True
            return

        logger.debug("telemetry.setup")

        # otlp
        if OTLP_ENDPOINT:
            from opentelemetry.exporter.otlp.proto.grpc.metric_exporter import OTLPMetricExporter
            from opentelemetry.exporter.otlp.proto.grpc.trace_exporter import OTLPSpanExporter

            resource = Resource(
                attributes={
                    SERVICE_NAME: get_from_env(
                        "SERVICE_NAME",
                        default="cli" if IS_DEV else None,
                        description="Service name",
                    ),
                    SERVICE_INSTANCE_ID: str(uuid4()),
                    SERVICE_VERSION: VERSION,
                    DEPLOYMENT_ENVIRONMENT: ENV,
                }
            )
            tracer_provider = TracerProvider(resource=resource)
            tracer_provider.add_span_processor(
                BatchSpanProcessor(OTLPSpanExporter(endpoint=OTLP_ENDPOINT, insecure=True))
            )
            trace.set_tracer_provider(tracer_provider)

            meter_provider = MeterProvider(
                resource=resource,
                metric_readers=[
                    PeriodicExportingMetricReader(
                        OTLPMetricExporter(endpoint=OTLP_ENDPOINT, insecure=True)
                    )
                ],
            )
            metrics.set_meter_provider(meter_provider)
            logger.debug("telemetry.otlp.setup", endpoint=OTLP_ENDPOINT)
        else:
            logger.debug("telemetry.oltp.disabled")

        _setup_telemetry = True

    def get_tracer(name: str) -> trace.Tracer:
        """Gets a tracer for the given name."""
        return trace.get_tracer(name)
except ImportError:

    class _FakeTracer:
        def __init__(self, name: str):
            self.name = name

        def start(self, name: str, *args, **kwargs):
            pass

        def end(self, *args, **kwargs):
            pass

        @contextmanager
        def start_as_current_span(self, name: str, *args, **kwargs):
            yield self

    def setup_telemetry() -> None:
        pass

    def get_tracer(name: str) -> "trace.Tracer":
        return _FakeTracer(name)  # type: ignore


setup_telemetry()
