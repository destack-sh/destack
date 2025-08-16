from .enum import OptionEnum, _to_casing, declare_enum, declare_option
from .universe import EnumType


@declare_enum(EnumType.STRING_CASING)
class StringCasing(OptionEnum):  # see :Casing
    SNAKE = declare_option(1, "snake_case")
    UPPER_CAMEL = declare_option(2, "UpperCamelCase")
    LOWER_CAMEL = declare_option(3, "lowerCamelCase")
    ALL_CAPS = declare_option(4, "ALL_CAPS")


def to_casing(name: str, casing: StringCasing, allow_whitespace: bool = False) -> str:
    """Convert a string to the given StringCasing."""
    return _to_casing(name, casing, allow_whitespace)  # type: ignore
