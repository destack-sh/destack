from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Callable, Collection, Optional, TypedDict, Union

from bench.language.const import BenchError

if TYPE_CHECKING:
    from bench.language import Property, TypeConstraint, TypeInfoBase
    from bench.language.value import SomeValue

ValidationSite = Union["TypeInfoBase", tuple["Property | Any", ...]]


class ValidationError(BenchError, ValueError):
    def __init__(
        self,
        value: Any,
        message: Optional[str],
        site: ValidationSite | None = None,
    ):
        super().__init__(f"{value!r}: {message}" + (f" at {site!r}" if site else ""))
        self.value = value
        self.site = site
        self.message = message


class ValidationOptions(TypedDict, total=False):
    properties: Collection["Property"] | Collection[Any]
    cause: Exception | None


ValidationHandler = Callable[["SomeValue", Optional[str], ValidationSite | None], None]


def on_invalid_raise(value: Any, message: Optional[str], site: ValidationSite | None):
    raise ValidationError(value, message, site)


@dataclass(slots=True)
class TypeConstraintIn:
    """A mini-TypeConstraint so we can define constraints without having to import :TypeConstraint."""

    min_value: float | None = None
    max_value: float | None = None
    step_value: float | None = None
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


SLUG_REGEX = r"^[a-z0-9-]{3,}$"
EMAIL_REGEX = r"^[a-zA-Z0-9_.+-]+@[a-zA-Z0-9-]+\.[a-zA-Z0-9-.]+$"
NAME_CONSTRAINT = TypeConstraintIn(min_length=1, max_length=128)
SLUG_CONSTRAINT = TypeConstraintIn(regex=SLUG_REGEX)
EMAIL_CONSTRAINT = TypeConstraintIn(regex=EMAIL_REGEX)


def constrain(
    min_value: float | None = None,
    max_value: float | None = None,
    step_value: float | None = None,
    min_length: int | None = None,
    max_length: int | None = None,
    regex: str | None = None,
) -> "TypeConstraintIn":
    return TypeConstraintIn(
        min_value=min_value,
        max_value=max_value,
        step_value=step_value,
        min_length=min_length,
        max_length=max_length,
        regex=regex,
    )
