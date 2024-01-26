import enum
import re
from typing import TYPE_CHECKING

import cachetools

from bench.language.const import BenchError

if TYPE_CHECKING:
    from bench.language.node import Struct, Node, Property


class ValidationError(BenchError, ValueError):
    def __init__(
        self,
        subject: "Struct",
        properties: list[str] | None,
        message: str,
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
        properties: list[str] | None,
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
        self.handler(self.subject, message, [self.prop.name], cause)


def on_invalid_raise(
    subject: "Node",
    message: str,
    properties: list[str] | None,
    cause: Exception | None = None,
):
    raise ValidationError(subject, properties, message, cause)


# :NameValidation
# names can be alphanumeric, hyphen, underscore, dot, spaces (but no tabs or newlines)
# leading and trailing spaces are fine

MAX_NAME_LENGTH = 256
NAME_REGEX = re.compile(r"^[a-zA-Z0-9_.\-:/ \xa0]*$")


def validate_name(value: str, on_issue: PropertyValidationHandler):
    if not isinstance(value, str):
        on_issue(f"not a string ({type(value)})")
    if len(value) > MAX_NAME_LENGTH:
        on_issue(f"too long ({len(value)} > {MAX_NAME_LENGTH})")
    if not NAME_REGEX.match(value):
        on_issue(f"invalid characters ('{value}')")


MAX_TEXT_LENGTH = 2048

# not used in packages right now?
MAX_DESCRIPTION_LENGTH = 512


# note: we cache these validators not for performance but for reference equality


@cachetools.cached({})
def enum_validator(t: type[enum.StrEnum | enum.IntEnum]):
    assert issubclass(t, (enum.StrEnum, enum.IntEnum)), f"invalid enum type: {t!r}"

    def validate_enum(value: str, on_issue: PropertyValidationHandler):
        if not isinstance(value, t) and value not in t.__members__:
            on_issue(f"invalid {t.__name__} ('{value}')")

    return validate_enum


@cachetools.cached({})
def flag_validator(t: type[enum.IntFlag]):
    assert issubclass(t, enum.IntFlag), f"invalid flag type: {t!r}"
    valid_mask = sum(t.__members__.values())

    def validate_flag(value: int, on_issue: PropertyValidationHandler):
        if not isinstance(value, t) and value & ~valid_mask:
            on_issue(f"invalid {t.__name__} ({value} & ~{valid_mask})")

    return validate_flag


@cachetools.cached({})
def isinstance_validator(t: type):
    def validate_isinstance(value: object, on_issue: PropertyValidationHandler):
        if not isinstance(value, t):
            on_issue(f"invalid type ({type(value)} != {t})")

    return validate_isinstance


validate_is_str = isinstance_validator(str)
validate_is_int = isinstance_validator(int)
validate_is_float = isinstance_validator(float)
validate_is_bool = isinstance_validator(bool)
