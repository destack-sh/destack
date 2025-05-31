from asyncio import CancelledError

from bench.language import BenchError, Code, Error, ErrorType, Text


class RuntimeError(BenchError, RuntimeError):
    def __init__(
        self, title: str | None = None, text: Text | None = None, error: Error | None = None
    ) -> None:
        super().__init__(title)
        self.title = title
        self.text = text
        self.error = error


class RetryableError(RuntimeError):
    """An error we can retry "immediately" at runtime (in the same runtime)."""


class NonRetryableError(RuntimeError):
    """An error we cannot retry "immediately" at runtime (in the same runtime)."""

    run_error_type = ErrorType.NON_RETRYABLE


class RunImpossibleError(NonRetryableError):
    run_error_type = ErrorType.RUN_IMPOSSIBLE


class NotSupportedError(RunImpossibleError):
    run_error_type = ErrorType.NOT_SUPPORTED


class InvalidValueError(RunImpossibleError):
    run_error_type = ErrorType.INVALID_VALUE


class CodeInvalidError(RunImpossibleError, SyntaxError):
    run_error_type = ErrorType.CODE_INVALID

    def __init__(
        self,
        title: str | None = None,
        text: Text | None = None,
        error: Error | None = None,
        code: Code | None = None,
    ) -> None:
        super().__init__(title, text, error)
        self.code = code


class AbortedError(CancelledError, NonRetryableError):
    run_error_type = ErrorType.ABORTED


class IncapableError(NonRetryableError):
    run_error_type = ErrorType.INCAPABLE


class RefusedError(NonRetryableError):
    run_error_type = ErrorType.REFUSED


class InterruptionCancelledError(RetryableError):
    run_error_type = ErrorType.INTERRUPTION_CANCELLED


class ModelFailedError(RetryableError):
    run_error_type = ErrorType.MODEL_FAILED
