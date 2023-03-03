from asyncio import CancelledError
from collections import OrderedDict
from typing import Coroutine, Iterable, Type, TypeVar, cast

import sentry_sdk
import structlog


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


logger = structlog.get_logger(__name__)


async def wrap_task(coro: Coroutine, task_id: str | None = None) -> None:
    task_id = task_id or coro.__name__
    try:
        logger.debug("start_task", task_id=task_id)
        return await coro
    except CancelledError as e:
        logger.exception("cancelled_task", task_id=task_id, exc_info=e)
        raise
    except Exception as e:
        logger.exception("errored_task", task_id=task_id, exc_info=e)
        if sentry_sdk.Hub.current is not None:
            sentry_sdk.capture_exception(e)
        raise
