import logging
import os

import posthog
import sentry_sdk
import structlog
from sentry_sdk.integrations.logging import LoggingIntegration

from bench.utils.env import IS_DEBUG, IS_TEST, SOME_TYPE_CHECKING
from bench.utils.utils import get_from_env

logger = structlog.get_logger(__name__)

IGNORED_PATHS = {"/", "/metrics", "/healthz", "/readiness", "/liveness"}


def traces_sampler(sampling_context: dict):
    if "asgi_scope" in sampling_context:
        path = sampling_context["asgi_scope"]["path"]
        if path in IGNORED_PATHS:
            return 0.0
    return 1.0  # by default sample everything


def init_sentry():
    sentry_sdk.utils.MAX_STRING_LENGTH = 10_000_000  # type: ignore
    # https://docs.sentry.io/platforms/python/
    sentry_logging = LoggingIntegration(level=logging.DEBUG, event_level=None)
    environment = os.getenv("SENTRY_ENVIRONMENT", "production")
    dsn = get_from_env("SENTRY_DSN")
    integrations = (sentry_logging,)

    sentry_sdk.init(
        dsn=dsn,
        environment=environment,
        integrations=integrations,
        sample_rate=1.0,
        send_default_pii=True,
        traces_sampler=traces_sampler,
    )
    logger.debug("sentry.initialized", environment=environment, dsn=dsn[:12] + "..." + dsn[-4:])


def setup_analytics():
    # Sentry
    if not (IS_TEST or IS_DEBUG or SOME_TYPE_CHECKING):
        init_sentry()

    # Posthog
    posthog.api_key = "phc_d8mi3OMdtKSVA8kzHbBoKtYU3ZsMQakAiLpuOn3W9ma"
    posthog.host = "https://eu.posthog.com"

    if IS_TEST or IS_DEBUG or SOME_TYPE_CHECKING:
        posthog.disabled = True
