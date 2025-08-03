import re
from enum import IntEnum
from typing import assert_never


class Casing(IntEnum):
    SNAKE = 1
    CAMEL = 2
    LOWER_CAMEL = 3
    ALL_CAPS = 4


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


def to_code_name(name: str) -> str:
    """
    Turns a string into a valid Python/JS/... identifier.
    Does not attempt to transform casing, just make as few transformations as possible.
    """
    if not name:
        return "_"
    elif name.isidentifier():
        return name
    else:
        name = "_".join(re.split(r"[^a-zA-Z0-9_]", name))
        if name[0].isdigit():
            name = "_" + name
        return name


_CACHED_CASING: dict[tuple[str, Casing, bool], str] = {}


def to_casing(name: str, casing: Casing, allow_whitespace: bool = False) -> str:
    """Turns a string into a valid Python identifier."""
    key = (name, casing, allow_whitespace)
    if key in _CACHED_CASING:
        return _CACHED_CASING[key]

    if casing == Casing.SNAKE:  # snake_case
        # first transform lowerUpper transitions into lower_upper
        name = re.sub(r"(?<=[a-z])(?=[A-Z])", "_", name)
        # turn non-alphanumeric characters into underscores
        name = re.sub(r"[^a-zA-Z0-9_]", "_", name)
        name = _strip_alpha_num(name)
        name = name.lower()
        if allow_whitespace:
            name = name.replace("_", " ").strip()
    elif casing == Casing.CAMEL or casing == Casing.LOWER_CAMEL:  # CamelCase or lowerCamelCase
        # if it's already a mix of uppercase and lowercase starting with uppercase, leave it alone
        if re.match(r"^[A-Z][a-z0-9]+([A-Z]+[a-z0-9]+)+", name):
            _CACHED_CASING[key] = name
            return name
        # ignore non-alphanumeric characters and capitalize the next character
        name = re.sub(r"[^a-zA-Z0-9]", " ", name)
        # split on existing uppercase characters and spaces
        name = " ".join(re.split(r"(?<=[a-z])(?=[A-Z])", name))
        name = _strip_alpha_num(name).title()
        name = name.replace("_", " ").strip() if allow_whitespace else name.replace(" ", "")
        if casing == Casing.LOWER_CAMEL:
            name = name[0].lower() + name[1:]
    elif casing == Casing.ALL_CAPS:  # ALL_CAPS
        # ALL_CAPS, ignore non-alphanumeric characters and capitalize the next character
        name = re.sub(r"[^a-zA-Z0-9]", " ", name)
        # split on existing uppercase characters and spaces
        name = " ".join(re.split(r"(?<=[a-z])(?=[A-Z])", name))
        name = _strip_alpha_num(name).upper().replace(" ", "_")
        if allow_whitespace:
            name = name.replace("_", " ").strip()
    else:
        assert_never(casing)

    _CACHED_CASING[key] = name
    return name


def is_valid_casing(name: str, casing: Casing, allow_whitespace: bool = False) -> bool:
    expected = to_casing(name, casing, allow_whitespace=allow_whitespace)
    return name == expected
