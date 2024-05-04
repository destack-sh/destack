from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Callable, Collection, Optional, TypedDict, Union

import betterproto

from bench.language.const import BenchError

if TYPE_CHECKING:
    from bench.language import Property, Struct, TypeConstraint
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


MIN_NAME_LENGTH = 1
MAX_NAME_LENGTH = 128
SLUG_REGEX = r"^[a-z0-9-]{3,}$"
EMAIL_REGEX = r"^[a-zA-Z0-9_.+-]+@[a-zA-Z0-9-]+\.[a-zA-Z0-9-.]+$"


@dataclass(slots=True)
class TypeConstraintIn:
    """A mini-TypeConstraint so we can define constraints without having to import TypeConstraint."""

    min_value: float | None = None
    max_value: float | None = None
    min_length: int | None = None
    max_length: int | None = None
    regex: str | None = None

    def into(self) -> "TypeConstraint":
        from bench.language.field import TypeConstraint

        return TypeConstraint(
            min_value=self.min_value,
            max_value=self.max_value,
            min_length=self.min_length,
            max_length=self.max_length,
            regex=self.regex,
        )


NAME_CONSTRAINT = TypeConstraintIn(min_length=MIN_NAME_LENGTH, max_length=MAX_NAME_LENGTH)
SLUG_CONSTRAINT = TypeConstraintIn(regex=SLUG_REGEX)
EMAIL_CONSTRAINT = TypeConstraintIn(regex=EMAIL_REGEX)
