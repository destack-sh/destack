import logging
import logging.config
import os
import warnings

import structlog

from bench.utils.utils import get_from_env

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

if not get_from_env("JSON_LOGS", default=False, type_cast=bool):
    logged_handlers = ["plain_console"]
else:
    logged_handlers = ["json_console"]

NOISY_LOG_SOURCES = {}
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
        "bench": {"handlers": logged_handlers, "level": LOG_LEVEL, "propagate": False},
        **NOISY_LOGGERS,
    },
}


def configure_logging(apply_logging: bool = True, apply_structlog: bool = True):
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
