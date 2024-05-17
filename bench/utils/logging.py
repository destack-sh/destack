import logging
import logging.config

import structlog

from bench.utils.utils import get_from_env

LOG_LEVEL = get_from_env("LOG_LEVEL", default="DEBUG")

# monkey patch structlog to add color support for custom 'trace' level
patched_styles = structlog.dev.ConsoleRenderer.get_default_level_styles()
patched_styles["trace"] = patched_styles["debug"]
structlog.dev.ConsoleRenderer.get_default_level_styles = lambda *args: patched_styles

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

if not get_from_env("JSON_LOGS", default=False, typ=bool):
    logged_handlers = ["plain_console"]
else:
    logged_handlers = ["json_console"]

LOGGING = {
    "version": 1,
    "disable_existing_loggers": False,
    "formatters": FORMATTERS,
    "handlers": HANDLERS,
    "loggers": {
        "bench": {"handlers": logged_handlers, "level": LOG_LEVEL, "propagate": False},
    },
}


def _format_duration(_, __, event_dict):
    if "duration" in event_dict:
        event_dict["duration"] = f"{event_dict['duration'] * 1000:.2f}ms"
    return event_dict


def configure_logging(apply_logging: bool = True, apply_structlog: bool = True):
    # add trace logging level
    TRACE = 5
    _add_logging_level("TRACE", logging.DEBUG - TRACE, "trace")
    structlog.stdlib.TRACE = TRACE  # type: ignore
    structlog.stdlib._NAME_TO_LEVEL["trace"] = TRACE  # type: ignore
    structlog.stdlib._LEVEL_TO_NAME[TRACE] = "trace"  # type: ignore

    def trace(self, msg, *args, **kw):
        return self.log(TRACE, msg, *args, **kw)

    for logger in structlog._log_levels._LEVEL_TO_FILTERING_LOGGER.values():  # type: ignore
        logger.trace = trace

    structlog.stdlib._FixedFindCallerLogger.trace = trace  # type: ignore
    structlog.stdlib.BoundLogger.trace = trace  # type: ignore

    if apply_logging:
        logging.config.dictConfig(LOGGING)
    if apply_structlog:
        structlog.configure(
            processors=[
                structlog.stdlib.filter_by_level,
                structlog.processors.TimeStamper(fmt="iso"),
                _format_duration,
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
        return conflict

    # Lock because logger class and level name are queried and set
    logging._acquireLock()  # type: ignore
    try:
        registered_num = logging.getLevelName(level_name)
        logger_class = logging.getLoggerClass()
        logger_adapter = logging.LoggerAdapter

        if registered_num != "Level " + level_name:
            items_found += 1
            items_conflict += check_conflict(
                registered_num != level_num,
                "Level {!r} already registered " "in logging module".format(level_name),
            )

        current_level = getattr(logging, level_name, _UNSET)
        if current_level is not _UNSET:
            items_found += 1
            items_conflict += check_conflict(
                current_level != level_num,
                "Level {!r} already defined " "in logging module".format(level_name),
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
    finally:
        logging._releaseLock()  # type: ignore
