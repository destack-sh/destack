import inspect
import os
import textwrap
import typing
from enum import Enum, StrEnum
from typing import Optional, SupportsIndex, Type, cast

import cachetools


def str_to_bool(value: str | None) -> bool:
    truthy_strs_lower = ("y", "yes", "t", "true", "on", "yup", "1")
    return value is not None and str(value).lower() in truthy_strs_lower


@cachetools.cached({}, key=lambda key, *args, **kwargs: key)
def get_from_env_maybe[T](
    key: str,
    *,
    description: str | None = None,
    default: Optional[T] = None,
    optional: bool = True,
    typ: type[T] = str,
) -> T | None:
    value = os.getenv(key)
    if value is None or value == "":
        if default is not None:
            return default
        elif optional:
            return None
        else:
            raise ValueError(
                f'environment variable {key} is required (type={typ}, description="{description}")'
            )
    try:
        if typ is bool:
            value = str_to_bool(value)
        elif issubclass(typ, StrEnum):
            try:
                value = typ(value)
            except ValueError:
                value = typ[value]
        elif issubclass(typ, Enum):
            try:
                value = int(value)  # type: ignore
                value = typ(value)
            except ValueError:
                if typ.__name__ in ("Region", "RegionZone", "RegionArea"):
                    # parse slug
                    value = cast(str, value).replace("_", "-").lower()
                    value = typ.get_by_slug(value)  # type: ignore
                else:
                    value = typ[cast(str, value).upper()]
        else:
            value = cast(T, typ(value))  # type: ignore
    except Exception as e:
        raise ValueError(
            f'environment variable is invalid (key={key}, value={value}, type_cast={typ}, description="{description}"'
        ) from e
    return cast(T, value)


def get_from_env[T](
    key: str,
    *,
    description: str | None = None,
    default: Optional[T] = None,
    typ: Type[T] = str,
) -> T:
    value = get_from_env_maybe(
        key, description=description, default=default, optional=False, typ=typ
    )
    return cast(T, value)


def get_list(text: str) -> list[str]:
    if not text:
        return []
    return [item.strip() for item in text.split(",")]


def get_method_source(method) -> str:
    cleaned_lines = []
    found_def = False
    for line in inspect.getsourcelines(method)[0]:
        if line.strip().startswith("@"):
            continue
        if "def " in line and not found_def:  # only omit first def
            found_def = True
            continue
        cleaned_lines.append(line)

    return textwrap.dedent("".join(cleaned_lines))


def omit_empty(obj):
    if isinstance(obj, dict):
        # avoid calling items because we need to override items in DotDict for values with items
        return {k: omit_empty(obj.get(k)) for k in obj if obj.get(k) is not None}
    elif isinstance(obj, list):
        return [omit_empty(v) for v in obj if v is not None]
    else:
        return obj


T = typing.TypeVar("T")


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


def format_python(code: str):
    try:
        import black

        return black.format_str(code, mode=black.Mode(line_length=100))  # type: ignore
    except ImportError:
        raise RuntimeError("black is required to format code") from None
    except Exception as e:
        raise ValueError(f"got bad code:\n{code}") from e
