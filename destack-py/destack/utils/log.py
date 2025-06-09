import enum
import logging
import logging.config
from dataclasses import dataclass
from io import StringIO
from time import time_ns
from typing import Any, Callable

import structlog
from opentelemetry import trace

from destack.utils.env import get_from_env


class LogMode(enum.StrEnum):
    PLAIN = "plain"
    JSON = "json"


_PYTHON_LOG_LEVEL_BY_LEVEL = {  # :LogLevel
    "TRACE": 5,
    "DEBUG": logging.DEBUG,
    "INFO": logging.INFO,
    "WARNING": logging.WARNING,
    "ERROR": logging.ERROR,
    "CRITICAL": logging.CRITICAL,
}


LOG_LEVEL = get_from_env(
    "LOG_LEVEL",
    default="DEBUG",
    description="Python log level [TRACE, DEBUG, INFO, WARNING, ERROR, CRITICAL]",
)
PYTHON_LOG_LEVEL = _PYTHON_LOG_LEVEL_BY_LEVEL[LOG_LEVEL]
LOG_MODE = get_from_env(
    "LOG_MODE", typ=LogMode, default=LogMode.JSON, description="Log mode [plain, json]"
)


def _padright(s: str, width: int) -> str:
    return s + " " * (width - len(s))


def _padleft(s: str, width: int) -> str:
    return " " * (width - len(s)) + s


@dataclass(slots=True)
class KeyValueColumnFormatter:
    """Extended KeyValueColumnFormatter"""

    key_style: str | None
    value_style: str
    reset_style: str
    value_repr: Callable[[object], str]
    width: int = 0
    pad: str = ">"
    prefix: str = ""
    postfix: str = ""

    def __call__(self, key: str, value: object) -> str:
        sio = StringIO()

        if self.prefix:
            sio.write(self.prefix)
            sio.write(self.reset_style)

        if self.key_style is not None:
            sio.write(self.key_style)
            sio.write(key)
            sio.write(self.reset_style)
            sio.write("=")

        sio.write(self.value_style)
        if self.pad == ">":
            sio.write(_padright(self.value_repr(value), self.width))
        else:
            sio.write(_padleft(self.value_repr(value), self.width))

        sio.write(self.reset_style)

        if self.postfix:
            sio.write(self.postfix)
            sio.write(self.reset_style)

        return sio.getvalue()


@dataclass(slots=True)
class DurationFormatter:
    """
    Formats duration with a fixed width and color corresponding to their length.
    Duration should be in nanoseconds.
    """

    level_styles: dict[str, str]
    reset_style: str
    width: int

    def __call__(self, key: str, value: object) -> str:
        if value == "":
            return _padright("", self.width)
        assert isinstance(value, int), f"expected int, got {value!r}"
        if value < 1_000_000:
            level = "debug"
        elif value < 100_000_000:
            level = "info"
        else:
            level = "warning"
        style = self.level_styles.get(level, "")
        duration_str = f"{value / 1_000_000:.3f}ms"
        return f"{style}{_padleft(duration_str, self.width)}{self.reset_style}"


# monkey patch structlog to add color support for custom 'trace' level
patched_styles = structlog.dev.ConsoleRenderer.get_default_level_styles()
patched_styles["trace"] = patched_styles["debug"]

# console style
structlog.dev.ConsoleRenderer.get_default_level_styles = lambda *args: patched_styles  # type: ignore
styles = structlog.dev._ColorfulStyles
LEVEL_STYLES = {
    "critical": styles.level_critical,
    "exception": styles.level_exception,
    "error": styles.level_error,
    "warn": styles.level_warn,
    "warning": styles.level_warn,
    "info": styles.level_info,
    "debug": styles.level_debug,
    "trace": styles.level_debug,
    "notset": styles.level_notset,
}
CONSOLE_FORMATTER = structlog.dev.ConsoleRenderer(
    columns=[
        # timestamp (dim)
        structlog.dev.Column(
            "timestamp",
            structlog.dev.KeyValueColumnFormatter(
                key_style=None,
                value_style=styles.timestamp,
                reset_style=styles.reset,
                value_repr=str,
            ),
        ),
        # level (color)
        structlog.dev.Column(
            "level",
            structlog.dev.LogLevelColumnFormatter(LEVEL_STYLES, reset_style=styles.reset, width=6),
        ),
        # event (very bright)
        structlog.dev.Column(
            "event",
            structlog.dev.KeyValueColumnFormatter(
                key_style=None,
                value_style=styles.bright,
                reset_style=styles.reset,
                value_repr=str,
                width=32,
            ),
        ),
        # duration
        structlog.dev.Column(
            "duration",
            DurationFormatter(level_styles=LEVEL_STYLES, reset_style=styles.reset, width=10),
        ),
        # logger (bright)
        structlog.dev.Column(
            "logger",
            structlog.dev.KeyValueColumnFormatter(
                key_style=None,
                value_style=styles.bright + styles.logger_name,
                reset_style=styles.reset,
                value_repr=str,
                prefix="[",
                postfix="]",
            ),
        ),
        # default formatter for the rest
        structlog.dev.Column(
            "",
            structlog.dev.KeyValueColumnFormatter(
                key_style=styles.kv_key,
                value_style=styles.kv_value,
                reset_style=styles.reset,
                value_repr=repr,
            ),
        ),
    ]
)

FORMATTERS = {
    "json_formatter": {
        "()": structlog.stdlib.ProcessorFormatter,
        "processor": structlog.processors.JSONRenderer(),
    },
    "plain_console": {
        "()": structlog.stdlib.ProcessorFormatter,
        "processor": CONSOLE_FORMATTER,
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
    "null": {
        "class": "logging.NullHandler",
    },
}

if LOG_MODE == LogMode.JSON:
    logged_handlers = ["json_console"]
elif LOG_MODE == LogMode.PLAIN:
    logged_handlers = ["plain_console"]
else:
    raise ValueError(f"unexpected log mode: {LOG_MODE}")

LOGGING = {
    "version": 1,
    "disable_existing_loggers": False,
    "formatters": FORMATTERS,
    "handlers": HANDLERS,
    "loggers": {
        "destack": {"handlers": logged_handlers, "level": LOG_LEVEL, "propagate": False},
    },
}


def trim_logger(_, __, event_dict: Any):
    """Removes the logger name from the event dict."""
    if "logger" in event_dict:
        event_dict["logger"] = event_dict["logger"].removeprefix("destack.")
    return event_dict


def inline_object_keys(_, __, event_dict: Any):
    """Inline key object attributes (so we get the repr + specific attributes to search)"""
    inlined = {}
    for key, value in event_dict.items():
        if id := getattr(value, "id", None):
            inlined[f"{key}_id"] = str(id)
        if ck := getattr(value, "ck", None):
            inlined[f"{key}_ck"] = str(ck)
    for key, value in inlined.items():
        event_dict[key] = value
    return event_dict


def trim_otel_span(_, __, event_dict: Any):
    """Removes the span, adds a duration if it's the current main span"""
    event_dict["duration"] = ""  # default to blank duration (for padding)
    if event_dict.get("span") == "current":
        span = trace.get_current_span()
        if hasattr(span, "start_time"):
            end_time = getattr(span, "end_time", None) or time_ns()
            duration = end_time - getattr(span, "start_time")
            event_dict["duration"] = duration
    if "span" in event_dict:
        del event_dict["span"]
    return event_dict


def inline_otel_span(_, __, event_dict: Any):
    """Always adds the current span"""
    span = trace.get_current_span()
    if "span" in event_dict:
        del event_dict["span"]
    trace_id = span.get_span_context().trace_id
    span_id = span.get_span_context().span_id
    if not trace_id or not span_id:
        return event_dict  # not enabled
    event_dict["trace_id"] = trace_id
    event_dict["span_id"] = span_id
    return event_dict


_TRACE_LEVEL = 5


def _trace(self, msg, *args, **kw):
    return self.log(_TRACE_LEVEL, msg, *args, **kw)


_setup_logging = False


def setup_logging(apply_logging: bool = True, apply_structlog: bool = True):
    global _setup_logging
    if _setup_logging:
        return

    # add trace logging level
    _add_logging_level("TRACE", logging.DEBUG - _TRACE_LEVEL, "trace")
    structlog.stdlib.TRACE = _TRACE_LEVEL  # type: ignore
    structlog.stdlib.NAME_TO_LEVEL["trace"] = _TRACE_LEVEL  # type: ignore
    structlog.stdlib.LEVEL_TO_NAME[_TRACE_LEVEL] = "trace"  # type: ignore
    structlog.stdlib._FixedFindCallerLogger.trace = _trace  # type: ignore
    structlog.stdlib.BoundLogger.trace = _trace  # type: ignore
    structlog.stdlib.AsyncBoundLogger.trace = _trace  # type: ignore
    structlog._native.LEVEL_TO_FILTERING_LOGGER[_TRACE_LEVEL] = (  # type: ignore
        structlog._native._make_filtering_bound_logger(_TRACE_LEVEL)  # type: ignore
    )
    for logger in structlog._native.LEVEL_TO_FILTERING_LOGGER.values():  # type: ignore
        logger.trace = _trace

    # apply logging
    if apply_logging:
        logging.config.dictConfig(LOGGING)
    if apply_structlog:
        pre_processors = [
            structlog.stdlib.add_log_level,
            structlog.stdlib.filter_by_level,
            structlog.stdlib.add_logger_name,
            structlog.processors.TimeStamper(fmt="iso", utc=True, key="timestamp"),
        ]
        if LOG_MODE == LogMode.PLAIN:
            custom_processors = [trim_logger, trim_otel_span]
        else:
            custom_processors = [inline_object_keys, inline_otel_span]
        post_processors = [
            structlog.stdlib.PositionalArgumentsFormatter(),
            structlog.processors.StackInfoRenderer(),
            structlog.processors.format_exc_info,
            structlog.processors.UnicodeDecoder(),
            structlog.stdlib.ProcessorFormatter.wrap_for_formatter,
        ]
        structlog.configure(
            processors=[*pre_processors, *custom_processors, *post_processors],
            context_class=dict,
            logger_factory=structlog.stdlib.LoggerFactory(),
            wrapper_class=structlog.make_filtering_bound_logger(PYTHON_LOG_LEVEL),
            cache_logger_on_first_use=True,
        )

    _setup_logging = True


_UNSET = object()


def _add_logging_level(
    level_name, level_num, method_name=None, *, exc_info=False, stack_info=False
):
    """
    Adds a logging level.
    Borrowed from https://gitlab.com/madphysicist/haggis/-/blob/master/src/haggis/logs.py
    """

    def for_logger_adapter(self, msg, *args, **kwargs):
        kwargs.setdefault("exc_info", exc_info)
        kwargs.setdefault("stack_info", stack_info)
        kwargs.setdefault("stacklevel", 2)
        self.log(level_num, msg, *args, **kwargs)

    def for_logger_class(self, msg, *args, **kwargs):
        if self.isEnabledFor(level_num):
            kwargs.setdefault("exc_info", exc_info)
            kwargs.setdefault("stack_info", stack_info)
            kwargs.setdefault("stacklevel", 2)
            self._log(level_num, msg, args, **kwargs)

    def for_logging_module(*args, **kwargs):
        kwargs.setdefault("exc_info", exc_info)
        kwargs.setdefault("stack_info", stack_info)
        kwargs.setdefault("stacklevel", 2)
        logging.log(level_num, *args, **kwargs)

    if not method_name:
        method_name = level_name.lower()
    if method_name == level_name:
        raise ValueError("Method name must differ from level name")

    # The number of items required for a full registration is 5
    items_found = 0
    # Items that are found complete but are not expected values
    items_conflict = 0

    def check_conflict(conflict, message):
        if conflict:
            raise AttributeError(message)
        return conflict

    def check_func_conflict(func, name, original_name, is_func, target):
        conflict = not (
            callable(func)
            and getattr(func, "_original_name", None) == original_name
            and getattr(func, "_exc_info", None) == exc_info
            and getattr(func, "_stack_info", None) == stack_info
        )
        return check_conflict(
            conflict,
            "{} {!r} already defined in {}".format(
                "Function" if is_func else "Method", name, target
            ),
        )

    # Lock because logger class and level name are queried and set
    registered_num = logging.getLevelName(level_name)  # type: ignore
    logger_class = logging.getLoggerClass()
    logger_adapter = logging.LoggerAdapter

    if registered_num != "Level " + level_name:
        items_found += 1
        items_conflict += check_conflict(
            registered_num != level_num,
            f"Level {level_name!r} already registered in logging module",
        )

    current_level = getattr(logging, level_name, _UNSET)
    if current_level is not _UNSET:
        items_found += 1
        items_conflict += check_conflict(
            current_level != level_num,
            f"Level {level_name!r} already defined in logging module",
        )

    logging_func = getattr(logging, method_name, _UNSET)
    if logging_func is not _UNSET:
        items_found += 1
        items_conflict += check_func_conflict(
            logging_func, method_name, for_logging_module.__name__, True, "logging module"
        )

    logger_method = getattr(logger_class, method_name, _UNSET)
    if logger_method is not _UNSET:
        items_found += 1
        items_conflict += check_func_conflict(
            logger_method, method_name, for_logger_class.__name__, False, "logger class"
        )

    adapter_method = getattr(logger_adapter, method_name, _UNSET)
    if adapter_method is not _UNSET:
        items_found += 1
        items_conflict += check_func_conflict(
            adapter_method, method_name, for_logger_adapter.__name__, False, "logger adapter"
        )

    # Make sure the method names are set to sensible values, but
    # preserve the names of the old methods for future verification.
    def label_func(func):
        func._original_name = func.__name__
        func.__name__ = method_name
        func._exc_info = exc_info
        func._stack_info = stack_info

    label_func(for_logging_module)
    label_func(for_logger_class)
    label_func(for_logger_adapter)

    # Actually add the new level
    logging.addLevelName(level_num, level_name)
    setattr(logging, level_name, level_num)
    setattr(logging, method_name, for_logging_module)
    setattr(logger_class, method_name, for_logger_class)
    setattr(logger_adapter, method_name, for_logger_adapter)
