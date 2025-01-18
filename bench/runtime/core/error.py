from asyncio import CancelledError

from bench.language import BenchError, Code, ComputedValue, Text
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


class NotSupportedError(RunImpossibleError):
    run_error_type = RunErrorType.NOT_SUPPORTED


class InvalidValueError(RunImpossibleError):
    run_error_type = RunErrorType.INVALID_VALUE


class InvalidComputedError(RunImpossibleError):
    run_error_type = RunErrorType.INVALID_COMPUTED

    def __init__(
        self,
        title: str | None = None,
        text: Text | None = None,
        error: RunError | None = None,
        computed_value: ComputedValue | None = None,
    ) -> None:
        super().__init__(title, text, error)
        self.computed_value = computed_value


class CodeInvalidError(RunImpossibleError, SyntaxError):
    run_error_type = RunErrorType.CODE_INVALID

    def __init__(
        self,
        title: str | None = None,
        text: Text | None = None,
        error: RunError | None = None,
        code: Code | None = None,
    ) -> None:
        super().__init__(title, text, error)
        self.code = code


class AbortedError(CancelledError, NonRetryableError):
    run_error_type = RunErrorType.ABORTED


class ModelIncapableError(NonRetryableError):
    run_error_type = RunErrorType.MODEL_INCAPABLE


class InterruptionCancelledError(RetryableError):
    run_error_type = RunErrorType.INTERRUPTION_CANCELLED


class ModelFailedError(RetryableError):
    run_error_type = RunErrorType.MODEL_FAILED
