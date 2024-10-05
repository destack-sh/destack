import asyncio
import base64
from asyncio import CancelledError
from datetime import date, datetime, time, timedelta
from typing import TYPE_CHECKING, Any
from uuid import UUID

from PIL.Image import Image

from bench import language
from bench.language import render
from bench.language.const import BenchError
from bench.language.file import upload
from bench.language.node import Node
from bench.language.path import get_node
from bench.language.run import RunError, RunErrorType, RunKind, RunOptions
from bench.language.setup import BENCH_CLASS_BY_NAME
from bench.runtime.capture import LogSink
from bench.utils.time import timedelta_from_isoformat, timedelta_to_isoformat

if TYPE_CHECKING:
    pass

RUN_ONCE = RunOptions(max_attempts=1)
BASE_RUN_OPTIONS_BY_KIND = {
    RunKind.CODE: RUN_ONCE,
    RunKind.TEXT: RunOptions(max_attempts=3),
    RunKind.STEP: RUN_ONCE,
    RunKind.FLOW: RUN_ONCE,
}


#
# Errors
#


class RuntimeError(BenchError, RuntimeError):
    pass


class RetryableError(RuntimeError):
    """An error we can retry "immediately" at runtime (in the same runtime)."""

    run_error_type = RunErrorType.UNKNOWN_RETRYABLE


class NonRetryableError(RuntimeError):
    run_error_type = RunErrorType.UNKNOWN_NONRETRYABLE


class ManualRetryableError(RetryableError):
    def __init__(self, error: RunError):
        super().__init__(error.title)
        self.error = error

    run_error_type = RunErrorType.MANUAL_RETRYABLE


class ManualNonRetryableError(NonRetryableError):
    def __init__(self, error: RunError):
        super().__init__(error.title)
        self.error = error

    run_error_type = RunErrorType.MANUAL_NONRETRYABLE


class RunImpossibleError(NonRetryableError):
    run_error_type = RunErrorType.RUN_IMPOSSIBLE


class InvalidValueError(RunImpossibleError):
    run_error_type = RunErrorType.INVALID_VALUE


class SyntaxError(RunImpossibleError, SyntaxError):
    run_error_type = RunErrorType.CODE_INVALID


class ReplayError(RunImpossibleError):
    run_error_type = RunErrorType.REPLAY


class AbortedError(CancelledError, NonRetryableError):
    run_error_type = RunErrorType.ABORTED


class ModelIncapableError(NonRetryableError):
    run_error_type = RunErrorType.MODEL_INCAPABLE


class ModelFailedError(RetryableError):
    run_error_type = RunErrorType.MODEL_FAILED


#
# Globals
#

# all bench types
STATIC_CODE_GLOBALS: dict[str, Any] = {
    **{k: v for k, v in vars(language).items() if not k.startswith("__")},
    **BENCH_CLASS_BY_NAME,
    "Image": Image,
    # some error types
    "NonRetryableError": NonRetryableError,
    "RetryableError": RetryableError,
    "RunImpossibleError": RunImpossibleError,
    "ModelIncapableError": ModelIncapableError,
    # external
    "asyncio": asyncio,
    "sleep": asyncio.sleep,
    # time
    "datetime": datetime,
    "date": date,
    "time": time,
    "timedelta": timedelta,
    "timedelta_to_isoformat": timedelta_to_isoformat,
    "timedelta_from_isoformat": timedelta_from_isoformat,
}
# and some general stuff
for t in (datetime, timedelta, UUID, base64):
    STATIC_CODE_GLOBALS[t.__name__] = t
DYNAMIC_CODE_GLOBALS: dict[str, Any] = {
    # dynamic globals are set per code run, these are just the types :CodeGlobals
    "self": Node,
    "get_node": get_node,
    "render": render,
    "upload": upload,
    "log": LogSink.log,
    "trace": LogSink.trace,
    "debug": LogSink.debug,
    "info": LogSink.info,
    "warn": LogSink.warn,
    "error": LogSink.error,
    "critical": LogSink.critical,
    "print": LogSink.print,
}
