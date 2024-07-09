import base64
from datetime import datetime, timedelta
from typing import TYPE_CHECKING, Any
from uuid import UUID

from bench import language
from bench.language.const import BenchError
from bench.language.run import RunErrorType, RunOptions
from bench.language.setup import BENCH_CLASS_BY_NAME

if TYPE_CHECKING:
    pass

DEFAULT_CODE_RUN_OPTIONS = RunOptions(max_attempts=1)
DEFAULT_TEXT_RUN_OPTIONS = RunOptions(max_attempts=3, retry_interval=3, backoff=2)
DEFAULT_FLOW_RUN_OPTIONS = RunOptions(max_attempts=1)

# all bench types
STATIC_CODE_GLOBALS: dict[str, Any] = {**vars(language), **BENCH_CLASS_BY_NAME}
# and some general stuff
for t in (datetime, timedelta, UUID, base64):
    STATIC_CODE_GLOBALS[t.__name__] = t
DYNAMIC_CODE_GLOBALS = {
    # dynamic globals are set per code run :CodeGlobals
    "self",
    "get_node",
    "log",
    "trace",
    "debug",
    "info",
    "warn",
    "error",
    "fatal",
    "print",
}


class BenchRuntimeError(BenchError, RuntimeError):
    pass


class NotRunnableError(BenchRuntimeError):
    run_error_type = RunErrorType.NOT_RUNNABLE


class RunHaltedError(BenchRuntimeError):
    pass
