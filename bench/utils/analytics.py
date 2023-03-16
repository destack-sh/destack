import logging
import os

import sentry_sdk
import structlog
from sentry_sdk.integrations.logging import LoggingIntegration

logger = structlog.get_logger(__name__)

IGNORED_PATHS = {"/", "/metrics", "/healthz", "/readiness", "/liveness"}
IGNORED_GRAPHQL_OPS = {"IntrospectionQuery", "systemInfo", "newNotifications"}


def traces_sampler(sampling_context: dict):
    if "asgi_scope" in sampling_context:
        path = sampling_context["asgi_scope"]["path"]
        if path in IGNORED_PATHS:
            return 0.0
    if "strawberry_context" in sampling_context:
        operation = sampling_context["strawberry_context"].operation_name
        if operation in IGNORED_GRAPHQL_OPS:
            return 0.0

    return 1.0  # by default sample everything


def init_sentry(*, django: bool):
    sentry_sdk.utils.MAX_STRING_LENGTH = 10_000_000
    # https://docs.sentry.io/platforms/python/
    sentry_logging = LoggingIntegration(level=logging.DEBUG, event_level=None)
    environment = os.getenv("SENTRY_ENVIRONMENT", "production")
    dsn = os.environ["SENTRY_DSN"]
    integrations = [sentry_logging]
    if django:
        from sentry_sdk.integrations.django import DjangoIntegration

        integrations.append(DjangoIntegration())

    sentry_sdk.init(
        dsn=dsn,
        environment=environment,
        integrations=integrations,
        request_bodies="always",
        sample_rate=1.0,
        send_default_pii=True,
        traces_sampler=traces_sampler,
        _experiments={
            "profiles_sample_rate": 1.0,
        },
    )
    logger.info(
        "initialized_sentry",
        django=django,
        environment=environment,
        dsn=dsn[:12] + "..." + dsn[-4:],
    )
