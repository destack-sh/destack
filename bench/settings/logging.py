import logging
import os
from pathlib import Path

import structlog

from bench.settings import get_from_env
from bench.settings.base import DEBUG, TEST

LOG_LEVEL = os.getenv("DJANGO_LOG_LEVEL", "DEBUG" if TEST or DEBUG else "INFO")
LOG_PATH: str = get_from_env("DJANGO_LOG_PATH", "logs")

FORMATTERS = {
    # "json_formatter": {
    #     "()": structlog.stdlib.ProcessorFormatter,
    #     "processor": structlog.processors.JSONRenderer(),
    # },
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
    "console": {
        "class": "logging.StreamHandler",
        "formatter": "plain_console",
    },
    "flat_line_file": {"class": "logging.NullHandler"},
    "null": {
        "class": "logging.NullHandler",
    },
}

if DEBUG and not TEST:
    # log to file and ensure corresponding directory exists
    Path(LOG_PATH).mkdir(exist_ok=True)
    # configure file handlers only if needed as all handlers are instantiated
    #  and file handlers fail is their path does not exist
    # HANDLERS["flat_line_file"] = {
    #     "class": "logging.handlers.RotatingFileHandler",
    #     "filename": os.path.join(LOG_PATH, "flat_line.log"),
    #     "formatter": "json_formatter",
    # }
    logged_handlers = ["console"]
else:
    logged_handlers = ["console"]

LOGGING = {
    "version": 1,
    "disable_existing_loggers": True,
    "formatters": FORMATTERS,
    "handlers": HANDLERS,
    "loggers": {
        "django_structlog": {"handlers": logged_handlers, "level": LOG_LEVEL, "propagate": False},
        "axes": {"handlers": logged_handlers, "level": LOG_LEVEL, "propagate": False},
        "bench": {"handlers": logged_handlers, "level": LOG_LEVEL, "propagate": False},
    },
}

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
    context_class=structlog.threadlocal.wrap_dict(dict),
    logger_factory=structlog.stdlib.LoggerFactory(),
    wrapper_class=structlog.make_filtering_bound_logger(logging.NOTSET),
    cache_logger_on_first_use=True,
)
