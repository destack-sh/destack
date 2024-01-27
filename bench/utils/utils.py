import inspect
import os
import sys
import textwrap
import typing
from dataclasses import field
from typing import TYPE_CHECKING, Any, Callable, Optional

import sentry_sdk


class UnreachableError(Exception):
    pass


def str_to_bool(value: str) -> bool:
    truthy_strs_lower = ("y", "yes", "t", "true", "on", "yup", "1")
    return value is not None and str(value).lower() in truthy_strs_lower


def get_from_env(
    key: str,
    default: Optional[Any] = None,
    *,
    alt: Optional[str] = None,
    optional: bool = False,
    type_cast: Optional[Callable] = None,
) -> Any:
    value = os.getenv(key)
    if alt and not value:
        value = os.getenv(alt)
    if value is None or value == "":
        if optional:
            return None
        elif default is not None:
            value = default
        else:
            raise ValueError(
                f'Environment variable "{key}" is required (alt="{alt}", type_cast={type_cast}).'
            )
    if type_cast is not None:
        if type_cast is bool:
            value = str_to_bool(value)
        else:
            value = type_cast(value)
    return value


def get_list(text: str) -> list[str]:
    if not text:
        return []
    return [item.strip() for item in text.split(",")]


def required_field(**kwargs):
    """Hacky way to make a field required when subclassing a dataclass with defaults."""

    _field = None

    def _raise_must_set():
        raise ValueError(f"field '{_field.name}' must be set")

    _field = field(default_factory=_raise_must_set, **kwargs, metadata={"required": True})
    return _field


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


def sentry_capture(e: Exception) -> bool:
    sentry_enabled = sentry_sdk.Hub.current is not None
    if sentry_enabled:
        sentry_sdk.capture_exception(e)
    return sentry_enabled


class DotList(list):
    """
    Access a list of dictionaries as a list of DotDicts.
    Attribute and item access (with string) are column slices.
    """

    def __getitem__(self, item):
        if isinstance(item, str):
            return [row[item] for row in self]
        return super().__getitem__(item)

    def __getattr__(self, name):
        return [row[name] for row in self]


def omit_empty(obj):
    if isinstance(obj, dict):
        # avoid calling items because we need to override items in DotDict for values with items
        return {k: omit_empty(obj.get(k)) for k in obj.keys() if obj.get(k) is not None}
    elif isinstance(obj, list):
        return [omit_empty(v) for v in obj if v is not None]
    else:
        return obj


T = typing.TypeVar("T")


class frozendict(dict):
    def __setitem__(self, key, value):
        raise TypeError("FrozenDict does not support item assignment")

    def __delitem__(self, key):
        raise TypeError("FrozenDict does not support item deletion")


def freeze_dict(d: dict):
    return frozendict(d)


def format_python(code: str):
    try:
        import black

        return black.format_str(code, mode=black.Mode(line_length=100))
    except ImportError:
        raise RuntimeError("black is required to format code") from None
    except Exception as e:
        raise ValueError(f"got bad code:\n{code}") from e


ENVIRONMENT = os.environ.get("ENVIRONMENT", "local")
IS_DEBUG: bool = get_from_env("DEBUG", False, type_cast=str_to_bool)
IS_TEST: bool = (
    "test" in sys.argv
    or "pytest" in sys.argv[0]
    or get_from_env("TEST", False, type_cast=str_to_bool)
)
IS_LOCAL = ENVIRONMENT == "local"
SOME_TYPE_CHECKING = TYPE_CHECKING or "mypy" in sys.argv[0] or IS_TEST
