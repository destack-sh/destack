from asyncio import CancelledError
from dataclasses import dataclass
from functools import wraps
from typing import Any, Awaitable, Callable, Coroutine, Type, TypeVar, Union

from grpclib import GRPCError
from grpclib import Status as GRPCStatus
from grpclib.exceptions import ProtocolError, StreamTerminatedError

from bench.utils.oracle import Oracle

T = TypeVar("T")


@dataclass(slots=True)
class RetryOptions:
    """Options for retrying an operation."""

    max_attempts: int = 3  # < 0 for infinite
    retry_interval: float = 1.0  # seconds
    backoff: float = 2.0  # exponential backoff
    max_retry_interval: float = 30.0  # seconds
    jitter: float | None = None  # [0, 1] percentage of randomness
    retry_on: Union[Type[Exception], tuple[Type[Exception], ...]] = Exception
    retry_if: Callable[[Exception], bool] | None = None

    def __str__(self) -> str:
        return f"max_attempt={self.max_attempts}, retry_interval={self.retry_interval}, backoff={self.backoff}, max_retry_interval={self.max_retry_interval}"

    def __repr__(self) -> str:
        return f"<RetryOptions {self}>"

    def get_wait_interval(self, attempt: int, oracle: Oracle) -> float:
        backoff = self.backoff ** (max(1, attempt) - 1)  # no backoff on first attempt
        interval = min(self.retry_interval * backoff, self.max_retry_interval)
        if self.jitter is not None:
            interval *= 1 + self.jitter * (2 * oracle.random.random() - 1)
        return interval

    def new(self, oracle: Oracle, attempt: int = 0):
        return RetryState(options=self, start_ns=oracle.time_ns(), oracle=oracle, attempt=attempt)


class RetryError(Exception):
    """Error raised when retrying an operation fails."""

    def __init__(self, state: "RetryState", operation: Any = None):
        if operation is not None:
            super().__init__(f"retry exhausted for {operation!r}: {state!r}")
        else:
            super().__init__(f"retry exhausted: {state!r}")
        self.state = state


@dataclass(slots=True)
class RetryState:
    """State of a retryable operation."""

    options: RetryOptions
    start_ns: float
    oracle: Oracle
    errors: list[Exception] | None = None
    attempt: int = 0

    def __str__(self) -> str:
        parts = [f"attempt={self.attempt}"]
        if self.errors:
            parts.append(f"errors={self.errors}")
        return ", ".join(parts)

    def __repr__(self) -> str:
        return f"<RetryState {self}>"

    def on_attempt(self):
        self.attempt += 1

    def on_success(self):
        self.attempt = 0
        self.errors = None

    def on_error(self, error: Exception) -> bool:
        if self.errors is None:
            self.errors = []
        self.errors.append(error)
        should_retry_on_error = isinstance(error, self.options.retry_on) and (
            self.options.retry_if is None or self.options.retry_if(error)
        )
        return should_retry_on_error

    @property
    def last_error(self) -> Exception | None:
        return self.errors[-1] if self.errors else None

    @property
    def should_retry(self) -> bool:
        attempts_left = self.options.max_attempts <= 0 or self.attempt < self.options.max_attempts
        bad_error = self.errors and any(
            not isinstance(e, self.options.retry_on)
            or (self.options.retry_if and not self.options.retry_if(e))
            for e in self.errors
        )
        return attempts_left and not bad_error

    @property
    def duration_ns(self) -> float:
        return self.oracle.time_ns() - self.start_ns

    def get_wait_interval(self) -> float:
        return self.options.get_wait_interval(self.attempt, self.oracle)

    def to_error(self, operation: Any = None) -> RetryError | Exception:
        if self.errors:
            return self.errors[-1]
        else:
            return RetryError(self, operation=operation)


def retry(
    options: Union[RetryOptions, Callable[..., RetryOptions]],
    oracle: Oracle,
    on_error: Callable[..., None] | None = None,
):
    """
    Retry the decorated coroutine function on certain exceptions.
    Simple decorator for when you have a fixed oracle.
    """

    def decorator(func: Callable[..., Awaitable[T]]) -> Callable[..., Coroutine[None, None, T]]:
        @wraps(func)
        async def wrapper(*args, **kwargs) -> T:
            opt = options(*args, **kwargs) if callable(options) else options
            retry = opt.new(oracle)
            while retry.should_retry:
                retry.on_attempt()
                try:
                    return await func(*args, **kwargs)
                except opt.retry_on as e:
                    retry.on_error(e)
                    if on_error is not None:
                        on_error(*args, **kwargs, e=e)
                    if not retry.should_retry:
                        raise
                    await oracle.sleep(retry.get_wait_interval())
            raise retry.to_error()

        return wrapper

    return decorator


def is_retryable_grpc_error(e: Exception) -> bool:
    return (
        isinstance(e, GRPCError)
        and e.status
        in (
            GRPCStatus.UNKNOWN,
            GRPCStatus.UNAVAILABLE,
            GRPCStatus.CANCELLED,
            GRPCStatus.ABORTED,
            GRPCStatus.DEADLINE_EXCEEDED,
            GRPCStatus.RESOURCE_EXHAUSTED,
        )
    ) or isinstance(
        e, (OSError, StreamTerminatedError, ProtocolError, RuntimeError, CancelledError)
    )


RETRY_STANDARD = RetryOptions()
RETRY_NEVER = RetryOptions(max_attempts=1)
RETRY_FOREVER = RetryOptions(max_attempts=-1)
RETRY_GRPC = RetryOptions(retry_if=is_retryable_grpc_error)
RETRY_GRPC_FOREVER = RetryOptions(retry_if=is_retryable_grpc_error, max_attempts=-1)
