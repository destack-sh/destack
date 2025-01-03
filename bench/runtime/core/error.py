from asyncio import CancelledError

from bench.language.core.const import BenchError
from bench.language.core.text import Text
from bench.language.runtime.run import RunError, RunErrorType


class RuntimeError(BenchError, RuntimeError):
    def __init__(
        self, title: str | None = None, text: Text | None = None, error: RunError | None = None
    ) -> None:
        super().__init__(title)
        self.title = title
        self.text = text
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


class InterruptionCancelledError(RetryableError):
    run_error_type = RunErrorType.INTERRUPTION_CANCELLED


class ModelFailedError(RetryableError):
    run_error_type = RunErrorType.MODEL_FAILED
