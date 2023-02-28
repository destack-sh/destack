import logging
import os

import posthog
import sentry_sdk
from sentry_sdk.integrations.django import DjangoIntegration
from sentry_sdk.integrations.logging import LoggingIntegration

from bench.settings import TEST


# Sentry
def init_sentry():
    if not TEST:
        sentry_sdk.utils.MAX_STRING_LENGTH = 10_000_000
        # https://docs.sentry.io/platforms/python/
        sentry_logging = LoggingIntegration(level=logging.INFO, event_level=None)
        sentry_sdk.init(
            dsn=os.environ["SENTRY_DSN"],
            environment=os.getenv("SENTRY_ENVIRONMENT", "production"),
            integrations=[DjangoIntegration(), sentry_logging],
            request_bodies="always",
            sample_rate=1.0,
            send_default_pii=True,
            traces_sampler=True,
        )


init_sentry()


# Posthog
posthog.project_api_key = "phc_d8mi3OMdtKSVA8kzHbBoKtYU3ZsMQakAiLpuOn3W9ma"
posthog.host = "https://eu.posthog.com"

if TEST:
    posthog.disabled = True
