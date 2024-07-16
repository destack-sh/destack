import base64
from datetime import datetime, timedelta
from typing import TYPE_CHECKING, Any
from uuid import UUID

from bench import language
from bench.language import render
from bench.language.const import BenchError
from bench.language.node import Node
from bench.language.path import get_node
from bench.language.run import RunErrorType, RunKind, RunOptions
from bench.language.setup import BENCH_CLASS_BY_NAME
from bench.runtime.capture import LogSink

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


class RunImpossibleError(NonRetryableError):
    run_error_type = RunErrorType.RUN_IMPOSSIBLE


class SyntaxError(RunImpossibleError, SyntaxError):
    pass


class ReplayError(RunImpossibleError):
    run_error_type = RunErrorType.REPLAY


class ModelFailedError(NonRetryableError):
    run_error_type = RunErrorType.MODEL_FAILED


class ModelIncapableError(RetryableError):
    run_error_type = RunErrorType.MODEL_INCAPABLE


#
# Globals
#

# all bench types
STATIC_CODE_GLOBALS: dict[str, Any] = {
    **{k: v for k, v in vars(language).items() if not k.startswith("__")},
    **BENCH_CLASS_BY_NAME,
    # some error types
    "NonRetryableError": NonRetryableError,
    "RetryableError": RetryableError,
    "RunImpossibleError": RunImpossibleError,
    "ModelIncapableError": ModelIncapableError,
}
# and some general stuff
for t in (datetime, timedelta, UUID, base64):
    STATIC_CODE_GLOBALS[t.__name__] = t
DYNAMIC_CODE_GLOBALS: dict[str, Any] = {
    # dynamic globals are set per code run, these are just the types :CodeGlobals
    "self": Node,
    "get_node": get_node,
    "render": render,
    "log": LogSink.log,
    "trace": LogSink.trace,
    "debug": LogSink.debug,
    "info": LogSink.info,
    "warn": LogSink.warn,
    "error": LogSink.error,
    "critical": LogSink.critical,
    "print": LogSink.print,
}
