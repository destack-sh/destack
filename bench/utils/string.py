from enum import IntEnum
from typing import assert_never

import cachetools
import regex


class Casing(IntEnum):
    SNAKE = 1
    CAMEL = 2
    LOWER_CAMEL = 3
    ALL_CAPS = 4


def _strip_alpha_num(name: str) -> str:
    # remove leading underscores
    name = regex.sub(r"^_+", "", name)
    # remove trailing underscores
    name = regex.sub(r"_+$", "", name)
    # remove double underscores
    name = regex.sub(r"__+", "_", name)
    # remove leading digits
    name = regex.sub(r"^[0-9]+", "", name)
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
        name = "_".join(regex.split(r"[^a-zA-Z0-9_]", name))
        if name[0].isdigit():
            name = "_" + name
        return name


@cachetools.cached(cache={})
def to_casing(name: str, casing: Casing, allow_whitespace: bool = False) -> str:
    """Turns a string into a valid Python identifier."""
    if casing == Casing.SNAKE:  # snake_case
        # first transform lowerUpper transitions into lower_upper
        name = regex.sub(r"(?<=[a-z])(?=[A-Z])", "_", name)
        # turn non-alphanumeric characters into underscores
        name = regex.sub(r"[^a-zA-Z0-9_]", "_", name)
        name = _strip_alpha_num(name)
        name = name.lower()
        if allow_whitespace:
            name = name.replace("_", " ").strip()
        return name
    elif casing == Casing.CAMEL or casing == Casing.LOWER_CAMEL:  # CamelCase or lowerCamelCase
        # if it's already a mix of uppercase and lowercase starting with uppercase, leave it alone
        if regex.match(r"^[A-Z][a-z0-9]+([A-Z]+[a-z0-9]+)+", name):
            return name
        # ignore non-alphanumeric characters and capitalize the next character
        name = regex.sub(r"[^a-zA-Z0-9]", " ", name)
        # split on existing uppercase characters and spaces
        name = " ".join(regex.split(r"(?<=[a-z])(?=[A-Z0-9])", name))
        name = _strip_alpha_num(name).title()
        name = name.replace("_", " ").strip() if allow_whitespace else name.replace(" ", "")
        if casing == Casing.LOWER_CAMEL:
            name = name[0].lower() + name[1:]
        return name
    elif casing == Casing.ALL_CAPS:  # ALL_CAPS
        # ALL_CAPS, ignore non-alphanumeric characters and capitalize the next character
        name = regex.sub(r"[^a-zA-Z0-9]", " ", name)
        # split on existing uppercase characters and spaces
        name = " ".join(regex.split(r"(?<=[a-z])(?=[A-Z0-9])", name))
        name = _strip_alpha_num(name).upper().replace(" ", "_")
        if allow_whitespace:
            name = name.replace("_", " ").strip()
        return name
    else:
        assert_never(casing)


def is_valid_casing(name: str, casing: Casing, allow_whitespace: bool = False) -> bool:
    expected = to_casing(name, casing, allow_whitespace=allow_whitespace)
    return name == expected


def humanize_number(num: float) -> str:
    """Formats numbers into their highest 3-exponent of 10 (k, m, b)"""
    if num < 1000:
        return str(num)
    elif num < 1000000:
        return f"{round(num / 100) / 10}k"
    elif num < 1000000000:
        return f"{round(num / 100000) / 10}m"
    else:
        return f"{round(num / 100000000) / 10}b"


def humanize_bytes(num: float, cutoff: float = 10, round_it: bool = True) -> str:
    """Shorten bytes into nearest (KB, MB, GB, etc.), keep up to 3 significant digits"""
    units = ["B", "KB", "MB", "GB", "TB", "PB"]
    unit = 0
    while num >= cutoff and unit < len(units) - 1:
        num /= 1024
        unit += 1
    if round_it:
        return f"{round(num)}{units[unit]}"
    elif unit == 0:
        return f"{num:.0f}{units[unit]}"
    elif num < 10:
        return f"{num:.1f}{units[unit]}"
    else:
        return f"{num:.0f}{units[unit]}"
