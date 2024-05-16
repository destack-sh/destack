import asyncio
import enum
import functools
import types
import typing
from asyncio import CancelledError
from collections import OrderedDict
from itertools import filterfalse, tee
from sys import intern
from typing import (
    Any,
    Callable,
    Collection,
    Coroutine,
    Iterable,
    Mapping,
    TypeVar,
    Union,
    cast,
)
from uuid import UUID

import structlog
from asgiref.sync import async_to_sync
from bitarray import bitarray
from cachetools import cached
from more_itertools import first

from bench.utils.utils import sentry_capture

logger = structlog.get_logger(__name__)


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
        raise TypeError(f"unexpected id type: {id!r}")


@cached(cache={})
def uuid_to_str(id: UUID | str | None) -> str | None:
    if not id:
        return None  # ignore empty strings
    elif isinstance(id, str):
        return id
    elif isinstance(id, UUID):
        return intern(str(id))
    else:
        raise TypeError(f"unexpected id type: {id!r}")


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


def partition[
    T
](pred: Callable[[T], bool], iterable: Iterable[T]) -> tuple[tuple[T, ...], tuple[T, ...]]:
    t1, t2 = tee(iterable)
    return tuple(filterfalse(pred, t1)), tuple(filter(pred, t2))


def group_by[K, V](iterable: Collection[V], key: typing.Callable[[V], K]) -> dict[K, list[V]]:
    """Groups an iterable by a key function"""
    result = {}
    for item in iterable:
        result.setdefault(key(item), []).append(item)
    return result


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


def auto_async_to_sync[T](func: typing.Callable[..., T]) -> typing.Callable[..., T]:
    """Automatically convert async functions to sync if not called in async context."""

    def decorate(func):
        # check that the func is async
        if not asyncio.iscoroutinefunction(func):
            raise TypeError(f"{func} is not a coroutine function")

        @functools.wraps(func)
        def wrapped(*args, **kwargs):
            # are we in an async context?
            try:
                asyncio.get_running_loop()
                is_in_loop = True
            except RuntimeError:
                is_in_loop = False
            if is_in_loop:
                return func(*args, **kwargs)
            else:
                return async_to_sync(func)(*args, **kwargs)

        return wrapped

    return decorate(func)


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


TypeAnnotation = typing.NamedTuple(
    "TypeAnnotation",
    [("type", type), ("is_union", bool), ("is_optional", bool), ("is_list", bool)],
)


def _resolve_py_type(py_type: type | str | typing.ForwardRef, type_map: dict[str, type]) -> type:
    """Resolves the py type if it's a forward ref"""
    if isinstance(py_type, str):
        return type_map[py_type]
    elif isinstance(py_type, typing.ForwardRef):
        return type_map[py_type.__forward_arg__]
    else:
        return py_type


def parse_py_annotation(
    py_type: type | str | typing.ForwardRef, type_map: dict[str, type]
) -> TypeAnnotation:
    """Parses the type information from a given py type. Uses type map to resolve forward refs."""
    is_union = False
    is_optional = False
    is_list = False
    if not isinstance(py_type, type):
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
    """Encodes an integer as a variable length quantity B64 string for concise."""
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


_MIN_ID_BY_ENUM: dict[type, int] = {}
_MAX_ID_BY_ENUM: dict[type, int] = {}

IdEnumT = TypeVar("IdEnumT", bound="IdEnum")


class IdEnum(enum.IntEnum):
    id: int
    ord: int

    def __new__(cls, id: int):
        obj = int.__new__(cls, id)
        obj._value_ = id
        obj.ord = len(cls)
        obj.id = id

        # check id
        assert id > 0, f"invalid id {id}"
        existing = first((v for v in cls if v.id == id), None)
        assert existing is None, f"{cls} has duplicate id {id} for {id} and {existing}"

        return obj

    @functools.cached_property
    def bench_name(self):
        from bench.utils.casing import Casing, to_casing

        return to_casing(self.name, Casing.CAMEL)

    @classmethod
    def get_min_id(cls) -> int:
        """Get the minimum id."""
        if cls not in _MIN_ID_BY_ENUM:
            _MIN_ID_BY_ENUM[cls] = min(v.id for v in cls)
        return _MIN_ID_BY_ENUM[cls]

    @classmethod
    def get_max_id(cls) -> int:
        """Get the maximum id."""
        if cls not in _MAX_ID_BY_ENUM:
            _MAX_ID_BY_ENUM[cls] = max(v.id for v in cls)
        return _MAX_ID_BY_ENUM[cls]

    @classmethod
    def get_min_ord(cls) -> int:
        """Get the minimum ord."""
        return 0

    @classmethod
    def get_max_ord(cls) -> int:
        """Get the maximum ord."""
        return len(cls)

    def to(self, combined_type: Union[IdEnumT, "IdEnumOrUnion"]) -> IdEnumT:
        return combined_type(self.id)  # type: ignore

    @staticmethod
    def combine(name: str, *enums: type["IdEnum"]) -> type["IdEnum"]:
        combined = IdEnum(name, {t.name: t.id for e in enums for t in e})
        return typing.cast(type["IdEnum"], combined)


IdEnumOrUnion = Union[IdEnum, Union[IdEnum, Any]]
# NOTE: IdEnumOrOnion is intended for stuff like AccessType = IdEnum.combine("AccessType", ReadType, ...)
#  But for type checking we have it as AccessType = ReadType | ...
#  So we make these methods accept 'Any' for compliance. Not great but it's a small footprint.
EnumT = TypeVar("EnumT", bound=IdEnum)


# noinspection PyPep8Naming
class bytetuple(typing.Generic[EnumT]):
    """
    Tuple with a bitarray for fast membership check.
    We accept only IdEnum instances because we use its ordinals for a compact bitarray.
    """

    def __init__(self, *items: EnumT, enum_cls: type[EnumT] | Union[EnumT, Any] | None = None):
        if len(items) == 1 and isinstance(items[0], Collection):
            items = tuple(items[0])
        self.tuple = items
        if enum_cls is None:
            assert len(items) > 0, "enum_cls or args is required"
            enum_cls = items[0].__class__
        assert isinstance(enum_cls, type) and issubclass(
            enum_cls, IdEnum
        ), f"invalid enum_cls: {enum_cls} ({items})"
        self.enum_cls = enum_cls
        self.bits = bitarray(enum_cls.get_max_ord() + 1)
        for arg in items:
            self.bits[arg.ord] = True

    def __bool__(self):
        return bool(self.tuple)

    def has(self, item: EnumT) -> bool:
        """Checks whether the item is of the correct type and is in the tuple."""
        assert isinstance(item, self.enum_cls), f"want {self.enum_cls}, got {item!r} ({type(item)})"
        return bool(self.bits[item.ord])

    __container__ = has

    def __and__(self, other: "bytetuple"):
        assert isinstance(other, bytetuple), f"invalid type: {type(other)}"
        assert (
            self.enum_cls == other.enum_cls
        ), f"invalid enum_cls: {self.enum_cls} != {other.enum_cls}"
        combined = self.bits & other.bits
        items = tuple(self.enum_cls(v) for v in range(len(combined)) if combined[v])
        return bytetuple(*items)

    def __or__(self, other: "bytetuple"):
        assert isinstance(other, bytetuple), f"invalid type: {type(other)}"
        assert (
            self.enum_cls == other.enum_cls
        ), f"invalid enum_cls: {self.enum_cls} != {other.enum_cls}"
        combined = self.bits | other.bits
        items = tuple(self.enum_cls(v) for v in range(len(combined)) if combined[v])
        return bytetuple(*items)

    def __iter__(self):
        return iter(self.tuple)

    def __len__(self):
        return len(self.tuple)

    def __getitem__(self, index):
        return self.tuple[index]

    def __repr__(self):
        return f"{self.__class__.__name__}({self.tuple})"

    def __str__(self):
        return f"{self.__class__.__name__}({self.tuple})"
