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
DEFAULT_RUN_OPTIONS_BY_KIND = {
    RunKind.CODE: RUN_ONCE,
    RunKind.TEXT: RunOptions(max_attempts=3),
    RunKind.STEP: RUN_ONCE,
    RunKind.FLOW: RUN_ONCE,
}

# all bench types
STATIC_CODE_GLOBALS: dict[str, Any] = {
    **{k: v for k, v in vars(language).items() if not k.startswith("__")},
    **BENCH_CLASS_BY_NAME,
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


class BenchRuntimeError(BenchError, RuntimeError):
    pass


class NotRunnableError(BenchRuntimeError):
    run_error_type = RunErrorType.NOT_RUNNABLE


class CodeSyntaxError(NotRunnableError, SyntaxError):
    pass


class HaltedError(BenchRuntimeError):
    pass


class ReplayError(NotRunnableError):
    run_error_type = RunErrorType.REPLAY


class ModelFailedError(BenchRuntimeError):
    run_error_type = RunErrorType.MODEL_FAILED
