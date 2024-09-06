from typing import TYPE_CHECKING, Any

import posthog
import structlog

from bench.utils.env import IS_DEV, IS_TEST
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


def setup_analytics():
    # Posthog
    posthog.api_key = "phc_d8mi3OMdtKSVA8kzHbBoKtYU3ZsMQakAiLpuOn3W9ma"
    posthog.host = "https://eu.posthog.com"

    if IS_TEST or IS_DEV or TYPE_CHECKING:
        posthog.disabled = True
