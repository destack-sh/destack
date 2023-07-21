import logging
import logging.config
import os

import structlog

from bench.utils.utils import DEBUG, TEST

LOG_LEVEL = os.getenv("DJANGO_LOG_LEVEL", "DEBUG")
NOISY_LOG_LEVEL = os.getenv("NOISY_LOG_LEVEL", "INFO")

FORMATTERS = {
    "json_formatter": {
        "()": structlog.stdlib.ProcessorFormatter,
        "processor": structlog.processors.JSONRenderer(),
    },
    "plain_console": {
        "()": structlog.stdlib.ProcessorFormatter,
        "processor": structlog.dev.ConsoleRenderer(pad_event=0),
    },
    "key_value": {
        "()": structlog.stdlib.ProcessorFormatter,
        "processor": structlog.processors.KeyValueRenderer(
            key_order=["timestamp", "level", "event", "logger"]
        ),
    },
}

HANDLERS = {
    "plain_console": {
        "class": "logging.StreamHandler",
        "formatter": "plain_console",
    },
    "json_console": {
        "class": "logging.StreamHandler",
        "formatter": "json_formatter",
    },
    "flat_line_file": {"class": "logging.NullHandler"},
    "null": {
        "class": "logging.NullHandler",
    },
}

if DEBUG and not TEST:
    logged_handlers = ["plain_console"]
else:
    logged_handlers = ["json_console"]

NOISY_LOG_SOURCES = {
    "bench.msg.core",
    "bench.api.runtime",
    "bench.api.job",
    "bench.api.build",
    "bench.api.execution",
    "bench.api.user",
    "bench.api.multiplayer",
    "bench.language.code",
    "bench.language.tracing",
    "bench.worker.run",
}
NOISY_LOGGERS = {
    source: {
        "handlers": logged_handlers,
        "level": NOISY_LOG_LEVEL,
        "propagate": False,
    }
    for source in NOISY_LOG_SOURCES
}

LOGGING = {
    "version": 1,
    "disable_existing_loggers": False,
    "formatters": FORMATTERS,
    "handlers": HANDLERS,
    "loggers": {
        "daphne": {"handlers": logged_handlers, "level": "INFO", "propagate": False},
        "django_structlog": {"handlers": logged_handlers, "level": LOG_LEVEL, "propagate": False},
        "axes": {"handlers": logged_handlers, "level": LOG_LEVEL, "propagate": False},
        "bench": {"handlers": logged_handlers, "level": LOG_LEVEL, "propagate": False},
        "django.db.backends": {
            "handlers": logged_handlers,
            "level": NOISY_LOG_LEVEL,
            "propagate": False,
        },
        **NOISY_LOGGERS,
    },
}


def configure_logging(apply_logging: bool, apply_structlog: bool):
    if apply_logging:
        logging.config.dictConfig(LOGGING)
    if apply_structlog:
        structlog.configure(
            processors=[
                structlog.stdlib.filter_by_level,
                structlog.processors.TimeStamper(fmt="iso"),
                structlog.stdlib.add_logger_name,
                structlog.stdlib.add_log_level,
                structlog.stdlib.PositionalArgumentsFormatter(),
                structlog.processors.StackInfoRenderer(),
                structlog.processors.format_exc_info,
                structlog.processors.UnicodeDecoder(),
                structlog.stdlib.ProcessorFormatter.wrap_for_formatter,
            ],
            context_class=dict,
            logger_factory=structlog.stdlib.LoggerFactory(),
            wrapper_class=structlog.make_filtering_bound_logger(logging.NOTSET),
            cache_logger_on_first_use=True,
        )
