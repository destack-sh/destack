import asyncio
import enum
import functools
import hashlib
import json
import math
import secrets
import types
import typing
from collections import OrderedDict
from itertools import cycle, filterfalse, islice, product, tee
from os import urandom
from sys import intern
from typing import (
    Any,
    Awaitable,
    Callable,
    Collection,
    Iterable,
    Mapping,
    assert_never,
    cast,
)
from uuid import UUID

import regex
import structlog
from cachetools import cached

from bench.utils.base58 import base58_encode

logger = structlog.get_logger(__name__)


def async_shield[T](coro: Callable[..., Awaitable[T]]) -> Callable[..., Awaitable[T]]:
    """Wraps an async function in a shield."""

    @functools.wraps(coro)
    async def wrapper(*args, **kwargs):
        return await asyncio.shield(coro(*args, **kwargs))

    return wrapper


def is_close(num_a: float | Any, num_b: float | Any, tol: float) -> bool:
    return (
        isinstance(num_a, (int, float))
        and isinstance(num_b, (int, float))
        and math.isclose(num_a, num_b, rel_tol=tol, abs_tol=tol)
    )


def stable_hash(*args) -> int:
    """
    Hashes a tuple of arguments deterministically into an int64.
    """
    hasher = hashlib.sha256()

    def update_hash(value):
        if isinstance(value, (list, tuple)):
            for item in value:
                update_hash(item)
        elif isinstance(value, (str, int, UUID, enum.Enum, type(None))):
            hasher.update(str(value).encode())
        elif isinstance(value, dict):
            hasher.update(json.dumps(value, sort_keys=True).encode())
        else:
            raise ValueError(f"cannot hash {value!r}")

    for arg in args:
        update_hash(arg)

    return int(hasher.hexdigest(), 16) % (1 << 63)


def try_to_uuid(id: UUID | str) -> UUID | str:
    if isinstance(id, UUID):
        return id
    try:
        return UUID(id)
    except (ValueError, TypeError):
        return id


@cached(cache={})
def to_uuid(id: str | UUID | None) -> UUID | None:
    if not id:
        return None  # ignore empty strings
    elif isinstance(id, str):
        try:
            return UUID(id)
        except ValueError as e:
            raise ValueError(f"invalid UUID: {id!r} ({type(id)})") from e
    elif isinstance(id, UUID):
        return id
    else:
        assert_never(id)


@cached(cache={})
def uuid_to_str(id: UUID | str | None) -> str | None:
    if not id:
        return None  # ignore empty strings
    elif isinstance(id, str):
        return id
    elif isinstance(id, UUID):
        return intern(str(id))
    else:
        assert_never(id)


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
        return next(iterator)  # type: ignore
    except StopIteration:
        return None


def partition[T](pred: Callable[[T], bool], iterable: Iterable[T]) -> tuple[list[T], list[T]]:
    t1, t2 = tee(iterable)
    return list(filterfalse(pred, t1)), list(filter(pred, t2))


def group_by[K, V](iterable: Collection[V], key: typing.Callable[[V], K]) -> dict[K, list[V]]:
    """Groups an iterable by a key function"""
    result = {}
    for item in iterable:
        result.setdefault(key(item), []).append(item)
    return result


def dict_product[K, V](input_dict: dict[K, list[V]]) -> list[dict[K, V]]:
    """
    Generate all possible combinations of key-value pairs from a dictionary
    where each key maps to a list of possible values.
    """
    if not input_dict:
        return []
    keys, values_lists = zip(*input_dict.items())
    if any(len(values) == 0 for values in values_lists):
        return []
    # Cartesian product of all value lists
    all_combinations = product(*values_lists)
    combination_dicts = [dict(zip(keys, combination)) for combination in all_combinations]
    return combination_dicts


def dict_zip_cycle[K, V](input_dict: dict[K, list[V]]) -> list[dict[K, V]]:
    """
    Zip the dictionary's value lists, cycling through shorter lists to match the length of the longest list.
    """
    if not input_dict:
        return []
    keys, values_lists = zip(*input_dict.items())
    if any(len(values) == 0 for values in values_lists):
        return []
    max_length = max(len(values) for values in values_lists)
    # make iterators for each list, cycling if necessary
    cycled_lists = []
    for values in values_lists:
        cycled = cycle(values)
        cycled_lists.append(islice(cycled, max_length))
    # zip the cycled lists and create dictionaries
    zipped = zip(*cycled_lists)
    combination_dicts = [dict(zip(keys, combination)) for combination in zipped]
    return combination_dicts


def dict_zip_latest[K, V](input_dict: dict[K, list[V]]) -> list[dict[K, V]]:
    """
    Zip the dictionary's value lists, using the last value of shorter lists to pad them up to the longest list.
    """
    if not input_dict:
        return []
    keys, values_lists = zip(*input_dict.items())
    if any(len(values) == 0 for values in values_lists):
        return []
    max_length = max(len(values) for values in values_lists)
    # pad
    padded_lists = []
    for values in values_lists:
        if len(values) < max_length:
            # Extend the list by repeating the last element
            extended = values + [values[-1]] * (max_length - len(values))
            padded_lists.append(extended)
        else:
            padded_lists.append(values)
    # zip the padded lists and create dictionaries
    zipped = zip(*padded_lists)
    combination_dicts = [dict(zip(keys, combination)) for combination in zipped]
    return combination_dicts


def try_tuple[T](obj: tuple[T, ...] | T | None) -> tuple[T, ...] | None:
    """To tuple if not None and not already a tuple"""
    if obj is None:
        return None
    if isinstance(obj, tuple):
        return obj
    return (obj,)


nextn = next_or_none


def dict_to_ordered[V](obj: dict[str, V]) -> OrderedDict[str, V]:
    if isinstance(obj, OrderedDict):
        return obj
    elif len(obj) > 1:
        raise ValueError("cannot order dict with multiple entries")
    else:
        return OrderedDict(**obj)


def dict_minus[K, V](obj: dict[K, V], *keys: Iterable[K]) -> dict[K, V]:
    return {k: v for k, v in obj.items() if k not in keys}


def dict_intersect[K, V](obj: dict[K, V], keys: Iterable[K]) -> dict[K, V]:
    return {k: v for k, v in obj.items() if k in keys}


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


class TypeAnnotation(typing.NamedTuple):
    type: type
    is_union: bool
    is_optional: bool
    is_list: bool


def _resolve_py_type(py_type: type | str | typing.ForwardRef, type_map: dict[str, type]) -> type:
    """Resolves the py type if it's a forward ref"""
    try:
        if isinstance(py_type, str):
            return type_map[py_type]
        elif isinstance(py_type, typing.ForwardRef):
            return type_map[py_type.__forward_arg__]
        else:
            return py_type
    except KeyError as e:
        raise ValueError(
            f"unresolved forward ref: {py_type} (known: {list(type_map.keys())})"
        ) from e


def parse_py_annotation(
    py_type: type | str | typing.ForwardRef, type_map: dict[str, type]
) -> TypeAnnotation:
    """Parses the type information from a given py type. Uses type map to resolve forward refs."""
    is_union = False
    is_optional = False
    is_list = False
    if not isinstance(py_type, type):
        if isinstance(py_type, typing.ForwardRef):
            py_type = py_type.__forward_arg__
        if isinstance(py_type, str):
            if py_type.endswith(" | None"):  # highly advanced parsing logic
                is_optional = True
                py_type = py_type[:-7]
        py_type = _resolve_py_type(py_type, type_map)
    # strip optional
    if typing.get_origin(py_type) in (typing.Union, types.UnionType):
        union_types = typing.get_args(py_type)
        # it's a true union if there's a non-None type
        actual_types = tuple(t for t in union_types if t is not type(None))
        is_union = len(actual_types) > 1
        is_optional = len(actual_types) < len(union_types)
        # reconstitute type annotation
        if is_union:
            actual_types = tuple(_resolve_py_type(t, type_map) for t in actual_types)
            py_type = cast(type, typing.Union[actual_types])  # type: ignore
        else:
            py_type = actual_types[0]
            py_type = _resolve_py_type(py_type, type_map)
    # strip list
    if typing.get_origin(py_type) in (list, tuple):
        py_type = typing.get_args(py_type)[0]
        py_type = _resolve_py_type(py_type, type_map)
        is_list = True
    return TypeAnnotation(py_type, is_union, is_optional, is_list)


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
    distances = [(s, levenshtein_distance(needle, s.lower())) for s in candidates]
    similar_strings = [
        string
        for string, distance in distances
        if distance <= len(needle) * 0.6 or needle in string
    ]
    return {string: candidates[string] for string in similar_strings}


def did_you_mean_str(candidates: dict[str, Any], needle: str, repr: bool = False) -> str | None:
    """
    Returns a string with a 'did you mean' suggestion.
    """
    similar_candidates = get_similar_strings(candidates, needle)
    if similar_candidates:
        if repr:
            similar_strs = [f"{k} {v}" for k, v in similar_candidates.items()]
        else:
            similar_strs = [f"`{k}`" for k in similar_candidates]
        # use , or for last item
        if len(similar_strs) > 1:
            similar_strs[-1] = f"or {similar_strs[-1]}"
        return f"Did you mean: {', '.join(similar_strs)}?"
    else:
        return None


def assert_collections_equal[T](a: Collection[T], b: Collection[T]):
    a = set(a)
    b = set(b)
    difference = a.symmetric_difference(b)
    if difference:
        raise ValueError(f"collections are not equal: {difference}")


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


def get_subclasses(cls, seen=None):
    """Gets all subclasses of a class recursively."""
    seen = seen if seen is not None else set()
    seen.add(cls)
    yield cls
    for subclass in cls.__subclasses__():
        if subclass not in seen:
            yield from get_subclasses(subclass, seen=seen)


BASE_64 = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"


def encode_b64vlq(value: int) -> str:
    """Encodes an integer as a variable length quantity B64 string for conciseness."""
    if value == 0:
        return "A"
    elif value < 0:
        raise ValueError(f"cannot encode negative value: {value}")
    else:
        result = ""
        while value:
            result += BASE_64[value & 63]
            value >>= 6
        return result


def decode_b64vlq(value: str) -> int:
    """Decodes a variable length quantity B64 string into an integer."""
    result = 0
    shift = 0
    for c in value:
        result += BASE_64.index(c) << shift
        shift += 6
    return result


def re_search_or_error(pattern: str, string: str) -> regex.Match[str]:
    match = regex.search(pattern, string)
    if match is None:
        raise ValueError(f"no match for {pattern!r} in {string!r}")
    return match


class dualmethod:  # noqa: N801
    """Decorator that can store both an instance and class version of the same method name."""

    def __init__(self, func=None):
        self._instance_func = func
        self._class_func = None

    def inst(self, func):
        self._instance_func = func
        return self

    def cls(self, func):
        self._class_func = func
        return self

    def __get__(self, instance, owner):
        if instance is None:
            # called on the class
            if self._class_func is None:
                raise AttributeError("No class method defined.")
            return self._class_func.__get__(owner, owner)
        else:
            # called on an instance
            if self._instance_func is None:
                raise AttributeError("No instance method defined.")
            return self._instance_func.__get__(instance, owner)


def sanitize_connection_uri(uri: str) -> str:
    return regex.sub(r":[^@]+@", ":*****@", uri)


def generate_access_token(length: int) -> str:
    """Generate a random access token."""
    bytes = urandom(length)
    return base58_encode(bytes)


def generate_encryption_key(length: int) -> str:
    """Encryption key for pgcrypto symmetric encryption."""
    return secrets.token_hex(length)


def generate_salt(length: int) -> bytes:
    """Generate a random salt."""
    return urandom(length)
