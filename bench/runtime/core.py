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
from bench.language.setup import BENCH_CLASS_BY_NAME, NODE_CLASS_STUBS_BY_NAME
from bench.runtime.capture import LogSink
from bench.utils.func import get_subclasses
from bench.utils.time import timedelta_from_isoformat, timedelta_to_isoformat

if TYPE_CHECKING:
    pass

ATTEMPT_ONCE = RunOptions(max_attempts=1)
ATTEMPT_THRICE = RunOptions(max_attempts=3)
BASE_RUN_OPTIONS_BY_KIND = {
    RunKind.CODE: ATTEMPT_ONCE,
    RunKind.ACTION: ATTEMPT_THRICE,
    RunKind.STEP: ATTEMPT_ONCE,
    RunKind.PIPE: ATTEMPT_ONCE,
    RunKind.FLOW: ATTEMPT_ONCE,
}


#
# Errors
#


class RuntimeError(BenchError, RuntimeError):
    def __init__(self, message: str | None = None, error: RunError | None = None) -> None:
        super().__init__(message)
        self.error = error


class RetryableError(RuntimeError):
    """An error we can retry "immediately" at runtime (in the same runtime)."""

    run_error_type = RunErrorType.RETRYABLE


class NonRetryableError(RuntimeError):
    """An error we cannot retry "immediately" at runtime (in the same runtime)."""

    run_error_type = RunErrorType.NON_RETRYABLE


class RunImpossibleError(NonRetryableError):
    run_error_type = RunErrorType.RUN_IMPOSSIBLE


class InvalidValueError(RunImpossibleError):
    run_error_type = RunErrorType.INVALID_VALUE


class CodeInvalidError(RunImpossibleError, SyntaxError):
    run_error_type = RunErrorType.CODE_INVALID


class ReplayError(RunImpossibleError):
    run_error_type = RunErrorType.REPLAY


class AbortedError(CancelledError, NonRetryableError):
    run_error_type = RunErrorType.ABORTED


class ModelIncapableError(NonRetryableError):
    run_error_type = RunErrorType.MODEL_INCAPABLE


class ActionChangedError(ModelIncapableError):
    run_error_type = RunErrorType.ACTION_CHANGED


class ModelFailedError(RetryableError):
    run_error_type = RunErrorType.MODEL_FAILED


#
# Globals
#

RUNTIME_ERROR_CLASSES = get_subclasses(RuntimeError)

# all bench types
STATIC_CODE_GLOBALS: dict[str, Any] = {
    **{k: v for k, v in vars(language).items() if not k.startswith("__")},
    **BENCH_CLASS_BY_NAME,
    **NODE_CLASS_STUBS_BY_NAME,
    "Image": Image,
    # some error types
    **{c.__name__: c for c in RUNTIME_ERROR_CLASSES},
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
