import asyncio
from asyncio import CancelledError
from collections import OrderedDict
from functools import wraps
from typing import Coroutine, Iterable, Type, TypeVar, cast

import structlog

from bench.utils.utils import sentry_capture_if_enabled

logger = structlog.get_logger(__name__)


def get_first(obj: dict, keys: Iterable[str]):
    """Gets the first non-None value out of a dict given a list of keys"""
    for key in keys:
        value = obj.get(key)
        if value is not None:
            return value
    return None


T = TypeVar("T")


def terrible_cast(cls: Type[T], obj) -> T:
    """
    Changes the actual class of an object.

    For obvious reasons, use this with great caution. This can lead to subtle and annoying bugs,
     but is also super convenient in rare circumstances.
    """
    obj.__class__ = cls
    return cast(T, obj)


K = TypeVar("K")
V = TypeVar("V")


def dict_to_ordered(obj: dict[K, V]) -> OrderedDict[K, V]:
    if isinstance(obj, OrderedDict):
        return obj

    if len(obj) > 1:
        raise ValueError("cannot order dict with multiple entries")

    return OrderedDict(**obj)


def dict_minus(obj: dict[K, V], keys: Iterable[K]) -> dict[K, V]:
    return {k: v for k, v in obj.items() if k not in keys}


async def wrap_task(coro: Coroutine, task_id: str | None = None) -> None:
    task_id = task_id or coro.__name__
    try:
        return await coro
    except CancelledError as e:
        logger.exception("cancelled_task", task_id=task_id, exc_info=e)
        raise
    except Exception as e:
        sentry_enabled = sentry_capture_if_enabled(e)
        logger.exception("errored_task", task_id=task_id, exc_info=e, sentry_enabled=sentry_enabled)
        raise


def debounce(delay: int, max_wait: int = None):
    """Debounces the async function by the given delay (in seconds) and
    ensures that the function is called at least once every max_wait seconds
    if provided"""

    def decorator(func):
        @wraps(func)
        async def debounced(*args, **kwargs):
            if debounced._task:
                debounced._task.cancel()

            async def call_it():
                await asyncio.sleep(delay)
                debounced._last_call_time = asyncio.get_event_loop().time()
                await func(*args, **kwargs)

            debounced._task = asyncio.ensure_future(call_it())

        debounced._task = None
        debounced._last_call_time = None
        return debounced

    return decorator
