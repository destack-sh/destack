from __future__ import annotations

from asyncio import CancelledError
from collections import OrderedDict
from typing import (
    Any,
    Collection,
    Coroutine,
    Iterable,
    Mapping,
    Type,
    TypeVar,
    cast,
)
from uuid import UUID

import structlog

from bench.utils.utils import sentry_capture_if_enabled

logger = structlog.get_logger(__name__)


def try_from_uuid(id: UUID | str) -> UUID | str:
    if isinstance(id, UUID):
        return id
    try:
        return UUID(id)
    except ValueError:
        return id


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


def dict_intersect(obj: dict[K, V], keys: Iterable[K]) -> dict[K, V]:
    return {k: v for k, v in obj.items() if k in keys}


async def wrap_task(coro: Coroutine, task_id: str | None = None) -> None:
    task_id = task_id or coro.__name__
    try:
        return await coro
    except CancelledError as e:
        logger.exception("task.cancelled", task_id=task_id, exc_info=e)
        raise
    except Exception as e:
        logger.exception(
            "task.errored", task_id=task_id, exc_info=e, sentry=sentry_capture_if_enabled(e)
        )
        raise


def describe_type(obj: Any) -> str:
    """
    Summarize the names (if available) and types of arguments.
    """
    if isinstance(obj, str):
        return f"str({len(obj)})"
    elif isinstance(obj, Mapping):
        return ", ".join(f"{name}={type(value).__name__}" for name, value in obj.items())
    elif isinstance(obj, Collection):
        return ", ".join(type(value).__name__ for value in obj)
    else:
        return type(obj).__name__


def levenshtein_distance(s1: str, s2: str) -> int:
    """
    Calculates the Levenshtein distance between two strings.
    """
    if len(s1) < len(s2):
        return levenshtein_distance(s2, s1)

    if len(s2) == 0:
        return len(s1)

    previous_row = range(len(s2) + 1)
    for i, c1 in enumerate(s1):
        current_row = [i + 1]
        for j, c2 in enumerate(s2):
            insertions = previous_row[j + 1] + 1
            deletions = current_row[j] + 1
            substitutions = previous_row[j] + (c1 != c2)
            current_row.append(min(insertions, deletions, substitutions))
        previous_row = current_row

    return previous_row[-1]


def get_similar_strings(candidates: dict[str, Any], needle: str) -> dict[str, Any]:
    """
    Returns a list of strings that are similar to the needle.
    Used for 'did you mean' suggestions.
    """
    needle = needle.lower()
    distances = [(s, levenshtein_distance(needle, s.lower())) for s in candidates.keys()]
    similar_strings = [
        string
        for string, distance in distances
        if distance <= len(needle) * 0.6 or needle in string
    ]
    return {string: candidates[string] for string in similar_strings}


def did_you_mean_str(candidates: dict[str, Any], needle: str) -> str:
    """
    Returns a string with a 'did you mean' suggestion.
    """
    similar_candidates = get_similar_strings(candidates, needle)
    if similar_candidates:
        similar_candidates_strs = [f"{k} {repr(v)}" for k, v in similar_candidates.items()]
        return f"did you mean: {', '.join(similar_candidates_strs)}  of {len(candidates)}?"
    return f"nothing similar in {len(candidates)} candidates"
