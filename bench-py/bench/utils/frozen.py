from typing import SupportsIndex


class frozendict(dict):  # noqa: FURB189, N801, RUF100
    def __setitem__(self, key, value):
        raise TypeError("FrozenDict does not support set")

    def __delitem__(self, key):
        raise TypeError("FrozenDict does not support del")

    def clear(self):
        raise TypeError("FrozenDict does not support clear")

    def pop(self, key, default=None):
        raise TypeError("FrozenDict does not support pop")


class frozenlist(list):  # noqa: FURB189, N801, RUF100
    def append(self, value):
        raise TypeError("FrozenList does not support append")

    def extend(self, value):
        raise TypeError("FrozenList does not support extend")

    def insert(self, index, value):
        raise TypeError("FrozenList does not support insert")

    def remove(self, value):
        raise TypeError("FrozenList does not support remove")

    def pop(self, index: SupportsIndex = -1):
        raise TypeError("FrozenList does not support pop")

    def clear(self):
        raise TypeError("FrozenList does not support clear")


def freeze_dict(d: dict):
    return frozendict(d)
