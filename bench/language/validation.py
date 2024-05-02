import enum
import re
from typing import TYPE_CHECKING, Any, Callable, Collection, Optional, TypedDict, Union

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
        message: Optional[str],
        options: Optional["ValidationOptions"] = None,
    ):
        properties = options.get("properties") if options else None
        super().__init__(f"{subject!r}: {message}" + (f" at {properties}" if properties else ""))
        self.subject = subject
        self.properties = properties
        self.message = message
        self.cause = options.get("cause") if options else None


class ValidationOptions(TypedDict, total=False):
    properties: Collection["Property"] | Collection[Any]
    cause: Exception | None


ValidationHandler = Callable[
    [
        "Struct",
        Optional[str],
        Optional[ValidationOptions],
    ],
    None,
]


def on_invalid_raise(
    subject: "Struct",
    message: Optional[str],
    options: Optional[ValidationOptions] = None,
):
    raise ValidationError(subject, message, options)


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
        self.handler(self.subject, message, {"properties": (self.prop,), "cause": cause})


MIN_NAME_LENGTH = 1
MAX_NAME_LENGTH = 128
SLUG_REGEX = r"^[a-z0-9-]{3,}$"
EMAIL_REGEX = r"^[a-zA-Z0-9_.+-]+@[a-zA-Z0-9-]+\.[a-zA-Z0-9-.]+$"


def validate_name(prop: "Property", value: str, invalid: PropertyValidationHandler):
    if not isinstance(value, str):
        invalid(f"not a string ({type(value)})")
    if len(value) < MIN_NAME_LENGTH:
        invalid(f"too short ({len(value)} < {MIN_NAME_LENGTH})")
    if len(value) > MAX_NAME_LENGTH:
        invalid(f"too long ({len(value)} > {MAX_NAME_LENGTH})")


def validate_slug(prop: "Property", value: str, invalid: PropertyValidationHandler):
    if not isinstance(value, str):
        invalid(f"not a string ({type(value)})")
    if not re.match(SLUG_REGEX, value):
        invalid(f"invalid slug ('{value}')")


def validate_email(prop: "Property", value: str, invalid: PropertyValidationHandler):
    if not isinstance(value, str):
        invalid(f"not a string ({type(value)})")
    if not re.match(EMAIL_REGEX, value):
        invalid(f"invalid email ('{value}')")


# TODO :Robustness: compile constraints into SQL?


# NOTE: we cache these validators not for performance but for reference equality


@cachetools.cached({})
def parent_validator():
    def validate_parent(prop: "Property", value: "Node", invalid: PropertyValidationHandler):
        assert prop.reference_nodes is not None, f"missing reference_nodes for {prop!r}"
        if value.metatype not in prop.reference_nodes:
            invalid(f"invalid parent type ({value.metatype} not in {prop.reference_nodes})")

    return validate_parent


@cachetools.cached({})
def enum_validator(t: type[enum.StrEnum | enum.IntEnum]):
    assert issubclass(t, (enum.StrEnum, enum.IntEnum)), f"invalid enum type: {t!r}"

    def validate_enum(prop: "Property", value: str, invalid: PropertyValidationHandler):
        if not isinstance(value, t) and value not in t.__members__:
            invalid(f"invalid {t.__name__} ('{value}')")

    return validate_enum


@cachetools.cached({})
def int_range_validator(min: int, max: int):
    assert min <= max, f"invalid range: {min} > {max}"

    def validate_range(prop: "Property", value: int, invalid: PropertyValidationHandler):
        if not isinstance(value, int):
            invalid(f"not an integer ({type(value)})")
        if value < min:
            invalid(f"too small ({value} < {min})")
        if value > max:
            invalid(f"too large ({value} > {max})")

    return validate_range


@cachetools.cached({})
def float_range_validator(min: float, max: float):
    assert min <= max, f"invalid range: {min} > {max}"

    def validate_range(prop: "Property", value: float, invalid: PropertyValidationHandler):
        if not isinstance(value, float):
            invalid(f"not a float ({type(value)})")
        if value < min:
            invalid(f"too small ({value} < {min})")
        if value > max:
            invalid(f"too large ({value} > {max})")

    return validate_range


@cachetools.cached({})
def flag_validator(t: type[enum.IntFlag]):
    assert issubclass(t, enum.IntFlag), f"invalid flag type: {t!r}"
    valid_mask = sum(t.__members__.values())

    def validate_flag(prop: "Property", value: int, invalid: PropertyValidationHandler):
        if not isinstance(value, t) and value & ~valid_mask:
            invalid(f"invalid {t.__name__} ({value} & ~{valid_mask})")

    return validate_flag
