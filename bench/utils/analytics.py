import logging
from typing import TYPE_CHECKING, Any

import posthog
import sentry_sdk
import structlog
from sentry_sdk.integrations.logging import LoggingIntegration

from bench.utils.env import ENV, IS_DEV, IS_TEST
from bench.utils.utils import get_from_env_maybe

logger = structlog.get_logger(__name__)

IGNORED_PATHS = {"/", "/metrics", "/healthz", "/readiness", "/liveness"}


def traces_sampler(sampling_context: Any):
    if "asgi_scope" in sampling_context:
        path = sampling_context["asgi_scope"]["path"]
        if path in IGNORED_PATHS:
            return 0.0
    return 1.0  # by default sample everything


SENTRY_DSN = get_from_env_maybe("SENTRY_DSN", description="Sentry DSN")


def init_sentry():
    sentry_sdk.utils.MAX_STRING_LENGTH = 10_000_000  # type: ignore
    # https://docs.sentry.io/platforms/python/
    sentry_logging = LoggingIntegration(level=logging.DEBUG, event_level=None)
    integrations = (sentry_logging,)

    sentry_sdk.init(
        dsn=SENTRY_DSN,
        environment=ENV,
        integrations=integrations,
        sample_rate=1.0,
        send_default_pii=True,
        traces_sampler=traces_sampler,
    )
    logger.debug("sentry.initialized", environment=ENV, dsn=SENTRY_DSN)


def setup_analytics():
    # Sentry
    if not (IS_TEST or IS_DEV or TYPE_CHECKING):
        init_sentry()

    # Posthog
    posthog.api_key = "phc_d8mi3OMdtKSVA8kzHbBoKtYU3ZsMQakAiLpuOn3W9ma"
    posthog.host = "https://eu.posthog.com"

    if IS_TEST or IS_DEV or TYPE_CHECKING:
        posthog.disabled = True
