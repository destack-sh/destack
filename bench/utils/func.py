from __future__ import annotations

import asyncio
import functools
import random
import secrets
import string
from asyncio import CancelledError
from collections import OrderedDict
from itertools import filterfalse, tee
import types
import typing
from typing import (
    Any,
    Collection,
    Coroutine,
    Iterable,
    Mapping,
    TypeVar,
)
from uuid import UUID

import structlog

from bench.utils.utils import sentry_capture

logger = structlog.get_logger(__name__)

T = TypeVar("T")

K = TypeVar("K")
V = TypeVar("V")


def try_to_uuid(id: UUID | str) -> UUID | str:
    if isinstance(id, UUID):
        return id
    try:
        return UUID(id)
    except (ValueError, TypeError):
        return id


def get_first(obj: dict, keys: Iterable[str]):
    """Gets the first non-None value out of a dict given a list of keys"""
    for key in keys:
        value = obj.get(key)
        if value is not None:
            return value
    return None


def next_or_none(iterator: Iterable[Any]) -> Any | None:
    """
    Returns the next item in the iterator, or None if the iterator is empty.
    """
    try:
        return next(iterator)
    except StopIteration:
        return None


def partition(pred, iterable) -> tuple[list[Any], list[Any]]:
    t1, t2 = tee(iterable)
    return list(filterfalse(pred, t1)), list(filter(pred, t2))


def try_tuple(obj: T) -> tuple[T, ...] | None:
    """To tuple if not None and not already a tuple"""
    if obj is None:
        return None
    if isinstance(obj, tuple):
        return obj
    return (obj,)


nextn = next_or_none


def dict_to_ordered(obj: dict[K, V]) -> OrderedDict[K, V]:
    if isinstance(obj, OrderedDict):
        return obj

    if len(obj) > 1:
        raise ValueError("cannot order dict with multiple entries")

    return OrderedDict(**obj)


def dict_minus(obj: dict[K, V], *keys: Iterable[K]) -> dict[K, V]:
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
    except BaseException as e:
        logger.exception("task.errored", task_id=task_id, exc_info=e, sentry=sentry_capture(e))
        raise


async def wait_then(delay: float, coro_or_func: Coroutine | callable, *args, **kwargs) -> None:
    """
    Wait for a delay, then call the given coroutine or function with the given arguments.
    """
    await asyncio.sleep(delay)
    if asyncio.iscoroutine(coro_or_func):
        await coro_or_func(*args, **kwargs)
    else:
        coro_or_func(*args, **kwargs)


def call_later(delay: float, coro_or_func: Coroutine | callable, *args, **kwargs) -> None:
    """
    Call the given coroutine or function with the given arguments after a delay.
    """
    loop = asyncio.get_event_loop()
    if asyncio.iscoroutine(coro_or_func):
        loop.call_later(delay, asyncio.create_task, coro_or_func(*args, **kwargs))
    else:
        loop.call_later(delay, coro_or_func, *args, **kwargs)


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


TypeInfo = typing.NamedTuple(
    "TypeInfo", [("is_optional", bool), ("is_arrayable", bool), ("is_array", bool)]
)


def strip_py_type(py_type: type) -> tuple[type, TypeInfo]:
    is_optional = False
    is_arrayable = False
    is_array = False
    # strip optional
    if typing.get_origin(py_type) in (typing.Union, types.UnionType):
        args = typing.get_args(py_type)
        if len(args) == 2 and args[1] == type(None):  # noqa: E721
            py_type = args[0]
            is_optional = True
        # convert x | list[x] as isarrayable
        elif len(args) == 2 and typing.get_origin(args[1]) is list:
            if args[0] != typing.get_args(args[1])[0]:
                raise ValueError(f"cannot map generic union types: {py_type}")
            py_type = args[0]
            is_arrayable = True
        else:
            raise ValueError(f"cannot map generic union types: {py_type}")
    # strip list
    if typing.get_origin(py_type) is list:
        py_type = typing.get_args(py_type)[0]
        is_array = True
    return py_type, TypeInfo(is_optional, is_arrayable, is_array)


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


def did_you_mean_str(candidates: dict[str, Any], needle: str, repr: bool = False) -> str:
    """
    Returns a string with a 'did you mean' suggestion.
    """
    similar_candidates = get_similar_strings(candidates, needle)
    if similar_candidates:
        if repr:
            similar_strs = [f"{k} {v}" for k, v in similar_candidates.items()]
        else:
            similar_strs = [f"‘{k}'" for k in similar_candidates.keys()]
        # use , or for last item
        if len(similar_strs) > 1:
            similar_strs[-1] = f"or {similar_strs[-1]}"
        return f"Did you mean: {', '.join(similar_strs)}?"
    return f"Nothing similar in {len(candidates)} candidates."


def cyrb53a(s: str, seed: int = 0) -> int:
    """
    53-bit cyrb53a hash.
    Reference: https://github.com/bryc/code/blob/master/jshash/experimental/cyrb53.js
    """
    h1 = 0xDEADBEEF ^ seed
    h2 = 0x41C6CE57 ^ seed

    for ch in s:
        ch_code = ord(ch)
        h1 = (h1 ^ ch_code) * 0x85EBCA77 & ((1 << 53) - 1)  # Limit to 53 bits
        h2 = (h2 ^ ch_code) * 0xC2B2AE3D & ((1 << 53) - 1)  # Limit to 53 bits

    h1 ^= ((h1 ^ (h2 >> 15)) * 0x735A2D97) & ((1 << 53) - 1)
    h2 ^= ((h2 ^ (h1 >> 15)) * 0xCAF649A9) & ((1 << 53) - 1)
    h1 ^= h2 >> 16
    h2 ^= h1 >> 16

    return ((h2 & ((1 << 32) - 1)) << 21) + (h1 >> 11)


def generate_random_name(length: int = 32, lowercase: bool = False) -> str:
    """Random alphanumeric name starting with alphabetic character."""
    if lowercase:
        pool = string.ascii_lowercase + string.digits
    else:
        pool = string.ascii_letters + string.digits
    name = random.choice(string.ascii_lowercase)
    name += "".join(random.choice(pool) for _ in range(length - 1))
    return name


generate_random_lowercase_name = functools.partial(generate_random_name, lowercase=True)


def generate_secret_password(length: int = 48) -> str:
    """URL-safe secret password."""
    password = secrets.token_urlsafe(length - 4)[: length - 4]
    # ensure at least one lowercase, uppercase, digit, special character
    password += random.choice(string.ascii_lowercase)
    password += random.choice(string.ascii_uppercase)
    password += random.choice(string.digits)
    password += random.choice("!@#$%^&*()_+-=")
    return password


def get_subclasses(cls, seen=None):
    seen = seen or set()
    seen.add(cls)
    for subclass in cls.__subclasses__():
        if subclass not in seen:
            yield from get_subclasses(subclass, seen=seen)
            yield subclass
