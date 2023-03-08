import logging
import os

import sentry_sdk
import structlog
from sentry_sdk.integrations.logging import LoggingIntegration

logger = structlog.get_logger(__name__)


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
        traces_sampler=True,
    )
    logger.info(
        "initialized_sentry",
        django=django,
        environment=environment,
        dsn=dsn[:12] + "..." + dsn[-4:],
    )
