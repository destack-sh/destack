import enum
import inspect
import os
import re
import sys
import textwrap
from dataclasses import field
from typing import Any, Callable, Optional

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


class IdentifierType(enum.StrEnum):
    METHOD = "method"
    TYPE = "type"
    CONSTANT = "constant"
    PATH = "path"
    VARIABLE = "variable"
    FIELD = "field"


def to_pyidentifier(name: str, type: IdentifierType) -> str:
    """Turns a string into a valid Python identifier."""
    if type in (
        IdentifierType.METHOD,
        IdentifierType.VARIABLE,
        IdentifierType.FIELD,
        IdentifierType.PATH,
    ):
        # snake_case, turn non-alphanumeric characters into underscores
        name = re.sub(r"[^a-zA-Z0-9_]", "_", name)
        name = _strip_alpha_num(name)
        return name.lower()
    elif type == IdentifierType.TYPE:
        # CamelCase, ignore non-alphanumeric characters and capitalize the next character
        name = re.sub(r"[^a-zA-Z0-9]", " ", name)
        # split on existing uppercase characters and spaces
        name = " ".join(re.split(r"(?<=[a-z])(?=[A-Z0-9])", name))
        return _strip_alpha_num(name).title().replace(" ", "")
    elif type == IdentifierType.CONSTANT:
        # ALL_CAPS, turn non-alphanumeric characters into underscores
        name = re.sub(r"[^a-zA-Z0-9_]", "_", name)
        name = _strip_alpha_num(name)
        return name.upper()


def _strip_alpha_num(name: str) -> str:
    # remove leading underscores
    name = re.sub(r"^_+", "", name)
    # remove trailing underscores
    name = re.sub(r"_+$", "", name)
    # remove double underscores
    name = re.sub(r"__+", "_", name)
    # remove leading digits
    name = re.sub(r"^[0-9]+", "", name)
    return name


def to_pyidentifier_multi(*parts: str, type: IdentifierType) -> str:
    return ".".join(to_pyidentifier(part, type) for part in parts)


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

    @classmethod
    def from_dict(cls, d):
        return cls(**d)


class DotDictList(list):
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


ENVIRONMENT = os.environ.get("ENVIRONMENT", "local")
DEBUG: bool = get_from_env("DEBUG", False, type_cast=str_to_bool)
TEST: bool = (
    "test" in sys.argv
    or "pytest" in sys.argv[0]
    or get_from_env("TEST", False, type_cast=str_to_bool)
)
LOCAL = os.environ.get("LOCAL_ENV", "local") == "local"
