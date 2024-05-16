import asyncio
from dataclasses import dataclass
from functools import wraps
from typing import Awaitable, Callable, Type, TypeVar, Union

T = TypeVar("T")


@dataclass(slots=True)
class RetryOptions:
    max_attempts: int = 3  # < 0 for infinite
    retry_interval: float = 1.0  # seconds
    backoff_factor: float = 2.0  # exponential backoff
    max_retry_interval: float = 60.0  # seconds
    retry_on: Union[Type[Exception], tuple[Type[Exception], ...]] = Exception


def retry(
    options: Union[RetryOptions, Callable[..., RetryOptions]],
    on_failure: Callable[..., Awaitable[T]] | None = None,
):
    """Retry the decorated coroutine function on certain exceptions."""

    def decorator(func: Callable[..., Awaitable[T]]) -> Callable[..., Awaitable[T]]:
        @wraps(func)
        async def wrapper(*args, **kwargs) -> T:
            nonlocal options
            if callable(options):
                options = options(*args, **kwargs)

            attempt = 0
            interval = options.retry_interval
            last_error = None

            while options.max_attempts < 0 or attempt < options.max_attempts:
                attempt += 1
                try:
                    return await func(*args, **kwargs)
                except options.retry_on as e:
                    last_error = e
                    if on_failure is not None:
                        on_failure(*args, **kwargs, e=e)
                    if options.max_attempts > 0 and attempt >= options.max_attempts:
                        raise
                    await asyncio.sleep(interval)
                    interval = min(interval * options.backoff_factor, options.max_retry_interval)

            raise last_error or RuntimeError(
                f"{attempt} attempts for {func.__name__} (options={options!r})"
            )

        return wrapper

    return decorator
