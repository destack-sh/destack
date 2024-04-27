import enum
import re
from typing import TYPE_CHECKING, Union

import betterproto
import cachetools

from bench.language.const import BenchError

if TYPE_CHECKING:
    from bench.language import Property
    from bench.language.node import Node, Struct
    from bench.proto.wire import AnyNodeData, AnyStructData


class ValidationError(BenchError, ValueError):
    def __init__(
        self,
        subject: Union["Struct", "AnyStructData", "AnyNodeData", betterproto.Message],
        message: str,
        properties: list["Property"] | None = None,
        cause: Exception | None = None,
    ):
        super().__init__(f"{subject!r}: {message}" + (f" at {properties}" if properties else ""))
        self.subject = subject
        self.properties = properties
        self.message = message
        self.cause = cause


class ValidationHandler:
    def __call__(
        self,
        subject: "Struct",
        message: str,
        properties: list["Property"] | None = None,
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
    properties: list["Property"] | None = None,
    cause: Exception | None = None,
):
    raise ValidationError(subject, message, properties, cause)


MIN_NAME_LENGTH = 1
MAX_NAME_LENGTH = 128
SLUG_REGEX = r"^[a-z0-9-]{3,}$"
EMAIL_REGEX = r"^[a-zA-Z0-9_.+-]+@[a-zA-Z0-9-]+\.[a-zA-Z0-9-.]+$"


def validate_name(prop: "Property", value: str, on_invalid: PropertyValidationHandler):
    if not isinstance(value, str):
        on_invalid(f"not a string ({type(value)})")
    if len(value) < MIN_NAME_LENGTH:
        on_invalid(f"too short ({len(value)} < {MIN_NAME_LENGTH})")
    if len(value) > MAX_NAME_LENGTH:
        on_invalid(f"too long ({len(value)} > {MAX_NAME_LENGTH})")


def validate_slug(prop: "Property", value: str, on_invalid: PropertyValidationHandler):
    if not isinstance(value, str):
        on_invalid(f"not a string ({type(value)})")
    if not re.match(SLUG_REGEX, value):
        on_invalid(f"invalid slug ('{value}')")


def validate_email(prop: "Property", value: str, on_invalid: PropertyValidationHandler):
    if not isinstance(value, str):
        on_invalid(f"not a string ({type(value)})")
    if not re.match(EMAIL_REGEX, value):
        on_invalid(f"invalid email ('{value}')")


# TODO :Robustness: compile constraints into SQL?


# NOTE: we cache these validators not for performance but for reference equality


@cachetools.cached({})
def parent_validator():
    def validate_parent(prop: "Property", value: "Node", on_invalid: PropertyValidationHandler):
        assert prop.reference_nodes is not None, f"missing reference_nodes for {prop!r}"
        if value.metatype not in prop.reference_nodes:
            on_invalid(f"invalid parent type ({value.metatype} not in {prop.reference_nodes})")

    return validate_parent


@cachetools.cached({})
def enum_validator(t: type[enum.StrEnum | enum.IntEnum]):
    assert issubclass(t, (enum.StrEnum, enum.IntEnum)), f"invalid enum type: {t!r}"

    def validate_enum(prop: "Property", value: str, on_invalid: PropertyValidationHandler):
        if not isinstance(value, t) and value not in t.__members__:
            on_invalid(f"invalid {t.__name__} ('{value}')")

    return validate_enum


@cachetools.cached({})
def int_range_validator(min: int, max: int):
    assert min <= max, f"invalid range: {min} > {max}"

    def validate_range(prop: "Property", value: int, on_invalid: PropertyValidationHandler):
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

    def validate_range(prop: "Property", value: float, on_invalid: PropertyValidationHandler):
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

    def validate_flag(prop: "Property", value: int, on_invalid: PropertyValidationHandler):
        if not isinstance(value, t) and value & ~valid_mask:
            on_invalid(f"invalid {t.__name__} ({value} & ~{valid_mask})")

    return validate_flag
