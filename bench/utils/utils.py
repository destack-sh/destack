import inspect
import os
import re
import sys
import textwrap
import typing
from dataclasses import field
from typing import TYPE_CHECKING, Any, Callable, Generator, Optional

import cachetools
import sentry_sdk

from bench.proto.core import ProtoStrEnum


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


# :IdentifierStrings


class IdentifierType(ProtoStrEnum):
    METHOD = "method", 1
    TYPE = "type", 2
    CONSTANT = "constant", 3
    PATH = "path", 4
    VARIABLE = "variable", 5
    FIELD = "field", 6


IdentT = IdentifierType


@cachetools.cached(cache={})
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
    elif type in (IdentifierType.TYPE,):
        # if it's already a mix of uppercase and lowercase starting with uppercase, leave it alone
        if re.match(r"^[A-Z][a-z0-9]+([A-Z]+[a-z0-9]+)+", name):
            return name
        # CamelCase, ignore non-alphanumeric characters and capitalize the next character
        name = re.sub(r"[^a-zA-Z0-9]", " ", name)
        # split on existing uppercase characters and spaces
        name = " ".join(re.split(r"(?<=[a-z])(?=[A-Z0-9])", name))
        return _strip_alpha_num(name).title().replace(" ", "")
    elif type in (IdentifierType.CONSTANT,):
        # ALL_CAPS, ignore non-alphanumeric characters and capitalize the next character
        name = re.sub(r"[^a-zA-Z0-9]", " ", name)
        # split on existing uppercase characters and spaces
        name = " ".join(re.split(r"(?<=[a-z])(?=[A-Z0-9])", name))
        return _strip_alpha_num(name).upper().replace(" ", "_")
    else:
        raise ValueError(f"unexpected identifier type: {type}")


def to_all_caps(name: str) -> str:
    # transform somethingNice into SOMETHING_NICE
    # if it's already all caps, leave it alone
    # ignore non-alphanumeric characters and capitalize the next character
    name = re.sub(r"[^a-zA-Z0-9]", " ", name)
    # split on existing uppercase characters and spaces
    name = " ".join(re.split(r"(?<=[a-z])(?=[A-Z0-9])", name))
    name = _strip_alpha_num(name).upper().replace(" ", "_")
    return name


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


@cachetools.cached(cache={})
def to_camel_case(snake_str: str) -> str:
    components = snake_str.split("_")
    return components[0] + "".join(x.capitalize() if x else "_" for x in components[1:])


@cachetools.cached(cache={})
def to_snake_case(camel_str: str) -> str:
    """From camel case to snake case."""
    components = re.split(r"(?<=[a-z])(?=[A-Z0-9])", camel_str)
    return "_".join(components).lower()


TO_KEBAB_CASE_RE = re.compile("((?<=[a-z0-9])[A-Z]|(?!^)[A-Z](?=[a-z]))")


def to_kebab_case(name: str) -> str:
    return TO_KEBAB_CASE_RE.sub(r"-\1", name).lower()


def capitalize_first(name: str) -> str:
    return name[0].upper() + name[1:]


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


def flatten(*lists: list[T] | tuple[T]) -> list[T] | tuple[T]:
    """Flatten a list, generator, element or mixed list of those."""
    # try to unwrap inner directly
    if len(lists) == 1:
        if isinstance(lists[0], (list, tuple)):
            if len(lists[0]) == 1:
                if isinstance(lists[0][0], (list, tuple)):
                    return lists[0][0]
                elif isinstance(lists[0][0], Generator):
                    return tuple(lists[0][0])
            return lists[0]
        elif isinstance(lists[0], Generator):
            return tuple(lists[0])

    # flatten out element by element
    flattened: list[T] = []
    for item in lists:
        if isinstance(item, (list, tuple)):
            flattened.extend(item)
        elif isinstance(item, Generator):
            flattened.extend(list(item))
        else:
            flattened.append(item)
    return flattened


class frozendict(dict):
    def __setitem__(self, key, value):
        raise TypeError("FrozenDict does not support item assignment")

    def __delitem__(self, key):
        raise TypeError("FrozenDict does not support item deletion")


def freeze_dict(d: dict):
    return frozendict(d)


SelfT = typing.TypeVar("SelfT")
P = typing.ParamSpec("P")
HybridT = typing.TypeVar("HybridT", covariant=True)


class hybridmethod(typing.Generic[SelfT, P, HybridT]):
    def __init__(
        self,
        func: Callable[
            typing.Concatenate[type[SelfT], P], HybridT
        ],  # Must be the classmethod version
    ):
        self.cls_func = func
        self.__doc__ = func.__doc__

    def instancemethod(self, func: Callable[typing.Concatenate[SelfT, P], HybridT]) -> typing.Self:
        self.instance_func = func
        return self

    def __get__(self, instance: Optional[SelfT], owner: typing.Type[SelfT]) -> Callable[P, HybridT]:
        if instance is None or self.instance_func is None:
            # either bound to the class, or no instance method available
            return self.cls_func.__get__(owner, None)
        return self.instance_func.__get__(instance, owner)


def identity(a: Any) -> Any:
    return a


def format_python(code: str):
    try:
        import black

        return black.format_str(code, mode=black.Mode(line_length=100))
    except ImportError:
        raise RuntimeError("black is required to format code") from None
    except Exception as e:
        raise ValueError(f"got bad code:\n{code}") from e


ENVIRONMENT = os.environ.get("ENVIRONMENT", "local")
DEBUG: bool = get_from_env("DEBUG", False, type_cast=str_to_bool)
TEST: bool = (
    "test" in sys.argv
    or "pytest" in sys.argv[0]
    or get_from_env("TEST", False, type_cast=str_to_bool)
)
LOCAL_ENV = os.environ.get("LOCAL_ENV", "local") == "local"
SOME_TYPE_CHECKING = TYPE_CHECKING or "mypy" in sys.argv[0] or TEST
