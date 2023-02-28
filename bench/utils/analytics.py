import logging
import os

import sentry_sdk
from sentry_sdk.integrations.django import DjangoIntegration
from sentry_sdk.integrations.logging import LoggingIntegration


def init_sentry():
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
