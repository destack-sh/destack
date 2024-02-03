import enum
import re
from typing import TYPE_CHECKING

import cachetools

from bench.language.const import BenchError

if TYPE_CHECKING:
    from bench.language.node import Struct, Node, Property, Property


class ValidationError(BenchError, ValueError):
    def __init__(
        self,
        subject: "Struct",
        message: str,
        properties: list["Property"] | None,
        cause: Exception | None = None,
    ):
        super().__init__(f"{subject!r}: {message} at {properties}")
        self.subject = subject
        self.properties = properties
        self.message = message
        self.cause = cause


class ValidationHandler:
    def __call__(
        self,
        subject: "Struct",
        message: str,
        properties: list["Property"] | None,
        cause: Exception | None = None,
    ):
        pass


class PropertyValidationHandler:
    def __init__(self, subject: "Struct", prop: "Property", handler: ValidationHandler):
        self.subject = subject
        self.handler = handler
        self.prop = prop

    def __call__(
        self,
        message: str,
        cause: Exception | None = None,
    ):
        message = f"{self.prop.name}: {message}"
        self.handler(self.subject, message, [self.prop], cause)


def on_invalid_raise(
    subject: "Node",
    message: str,
    properties: list["Property"] | None,
    cause: Exception | None = None,
):
    raise ValidationError(subject, message, properties, cause)


# :NameValidation
# names can be alphanumeric, hyphen, underscore, dot, spaces (but no tabs or newlines)
# leading and trailing spaces are fine

MAX_NAME_LENGTH = 256
NAME_REGEX = re.compile(r"^[a-zA-Z0-9_.\-:/ \xa0]*$")


def validate_name(value: str, on_invalid: PropertyValidationHandler):
    if not isinstance(value, str):
        on_invalid(f"not a string ({type(value)})")
    if len(value) > MAX_NAME_LENGTH:
        on_invalid(f"too long ({len(value)} > {MAX_NAME_LENGTH})")
    if not NAME_REGEX.match(value):
        on_invalid(f"invalid characters ('{value}')")


MAX_TEXT_LENGTH = 2048

# not used in packages right now?
MAX_DESCRIPTION_LENGTH = 512


# NOTE: we cache these validators not for performance but for reference equality
# TODO @Cleanup @Robustness: turn validators into Validators and compile constraints into SQL


@cachetools.cached({})
def enum_validator(t: type[enum.StrEnum | enum.IntEnum]):
    assert issubclass(t, (enum.StrEnum, enum.IntEnum)), f"invalid enum type: {t!r}"

    def validate_enum(value: str, on_invalid: PropertyValidationHandler):
        if not isinstance(value, t) and value not in t.__members__:
            on_invalid(f"invalid {t.__name__} ('{value}')")

    return validate_enum


@cachetools.cached({})
def int_range_validator(min: int, max: int):
    assert min <= max, f"invalid range: {min} > {max}"

    def validate_range(value: int, on_invalid: PropertyValidationHandler):
        if not isinstance(value, int):
            on_invalid(f"not an integer ({type(value)})")
        if value < min:
            on_invalid(f"too small ({value} < {min})")
        if value > max:
            on_invalid(f"too large ({value} > {max})")

    return validate_range


@cachetools.cached({})
def float_range_validator(min: float, max: float):
    assert min <= max, f"invalid range: {min} > {max}"

    def validate_range(value: float, on_invalid: PropertyValidationHandler):
        if not isinstance(value, float):
            on_invalid(f"not a float ({type(value)})")
        if value < min:
            on_invalid(f"too small ({value} < {min})")
        if value > max:
            on_invalid(f"too large ({value} > {max})")

    return validate_range


@cachetools.cached({})
def flag_validator(t: type[enum.IntFlag]):
    assert issubclass(t, enum.IntFlag), f"invalid flag type: {t!r}"
    valid_mask = sum(t.__members__.values())

    def validate_flag(value: int, on_invalid: PropertyValidationHandler):
        if not isinstance(value, t) and value & ~valid_mask:
            on_invalid(f"invalid {t.__name__} ({value} & ~{valid_mask})")

    return validate_flag
