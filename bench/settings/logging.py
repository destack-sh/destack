import logging
import os
from pathlib import Path

import structlog

from bench.settings import get_from_env
from bench.settings.base import DEBUG, TEST

LOGS_PATH: str = get_from_env("LOGS_PATH", "logs")
if DEBUG or TEST:
    # log to file and ensure corresponding directory exists
    logged_handlers = ["console", "flat_line_file"]
    Path(LOGS_PATH).mkdir(exist_ok=True)
else:
    logged_handlers = ["console"]

LOGGING = {
    "version": 1,
    "disable_existing_loggers": True,
    "formatters": {
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
    },
    "handlers": {
        "console": {
            "class": "logging.StreamHandler",
            "formatter": "plain_console",
        },
        "flat_line_file": {
            "class": "logging.handlers.RotatingFileHandler",
            "filename": os.path.join(LOGS_PATH, "flat_line.log"),
            "formatter": "key_value",
        },
    },
    "loggers": {
        "django_structlog": {
            "handlers": logged_handlers,
            "level": "INFO",
        },
        "bench": {
            "handlers": logged_handlers,
            "level": "INFO",
        },
    },
}

# use structlog logging only
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
