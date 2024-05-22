import asyncio
from dataclasses import dataclass
from functools import wraps
from typing import Awaitable, Callable, Coroutine, Type, TypeVar, Union

T = TypeVar("T")


@dataclass(slots=True)
class RetryOptions:
    max_attempts: int = 3  # < 0 for infinite
    retry_interval: float = 1.0  # seconds
    backoff: float = 2.0  # exponential backoff
    max_retry_interval: float = 60.0  # seconds
    retry_on: Union[Type[Exception], tuple[Type[Exception], ...]] = Exception

    def get_interval(self, attempt: int) -> float:
        return min(self.retry_interval * (self.backoff**attempt), self.max_retry_interval)


def retry(
    options: Union[RetryOptions, Callable[..., RetryOptions]],
    on_failure: Callable[..., Awaitable[T]] | None = None,
):
    """Retry the decorated coroutine function on certain exceptions."""

    def decorator(func: Callable[..., Awaitable[T]]) -> Callable[..., Coroutine[None, None, T]]:
        @wraps(func)
        async def wrapper(*args, **kwargs) -> T:
            if callable(options):
                _options = options(*args, **kwargs)
            else:
                _options = options

            attempt = 0
            interval = _options.retry_interval
            last_error = None

            while _options.max_attempts < 0 or attempt < _options.max_attempts:
                attempt += 1
                try:
                    return await func(*args, **kwargs)
                except _options.retry_on as e:
                    last_error = e
                    if on_failure is not None:
                        on_failure(*args, **kwargs, e=e)
                    if _options.max_attempts > 0 and attempt >= _options.max_attempts:
                        raise
                    await asyncio.sleep(interval)
                    interval = min(interval * _options.backoff, _options.max_retry_interval)

            raise last_error or RuntimeError(
                f"exceeded {attempt} attempts for {func.__name__} (options={options!r})"
            )

        return wrapper

    return decorator
