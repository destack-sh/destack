from collections import OrderedDict
from typing import Iterable, Type, TypeVar, cast


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
    # TODO @Robustness: don't do terrible casts
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
