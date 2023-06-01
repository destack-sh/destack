import inspect
import os
import re
import textwrap
from dataclasses import field
from typing import Any, Callable, Optional

import sentry_sdk


def str_to_bool(value: str) -> bool:
    truthy_strs_lower = ("y", "yes", "t", "true", "on", "yup", "1")
    return value is not None and str(value).lower() in truthy_strs_lower


def get_from_env(
    key: str,
    default: Optional[Any] = None,
    *,
    optional: bool = False,
    type_cast: Optional[Callable] = None,
) -> Any:
    value = os.getenv(key)
    if value is None or value == "":
        if optional:
            return None
        elif default is not None:
            value = default
        else:
            raise ValueError(f'The environment variable "{key}" is missing and required for Bench.')
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

    _field = field(default_factory=_raise_must_set, **kwargs)
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


def to_pyidentifier(name: str) -> str:
    return re.sub(r"\W|^(?=\d)", "_", name)


def to_pyidentifier_multi(*parts: str) -> str:
    return ".".join(to_pyidentifier(part) for part in parts)


def sentry_capture_if_enabled(e: Exception) -> bool:
    sentry_enabled = sentry_sdk.Hub.current is not None
    if sentry_enabled:
        sentry_sdk.capture_exception(e)
    return sentry_enabled


class DotDict(dict):
    """Access dictionary keys as attributes."""

    def __getattr__(self, name):
        try:
            return self[name]
        except KeyError:
            raise AttributeError(name)

    def __setattr__(self, name, value):
        self[name] = value
