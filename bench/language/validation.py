import dataclasses
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Callable, Collection, Optional, TypedDict, Union

import regex

from bench.language.const import BenchError, BlockType

if TYPE_CHECKING:
    from bench.language import (
        FileFormat,
        FileType,
        Property,
        StepType,
        TypeConstraint,
        TypeInfoBase,
    )
    from bench.language.value import SomeValue

ValidationSite = Union["TypeInfoBase", tuple["Property | Any", ...]]


class ValidationError(BenchError, ValueError):
    def __init__(
        self,
        value: Any,
        message: Optional[str],
        site: ValidationSite | None = None,
    ):
        if value is not None:
            super().__init__(f"{value!r}: {message}" + (f" at {site!r}" if site else ""))
        else:
            super().__init__(f"{message}" + (f" at {site!r}" if site else ""))
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
class ValidationCollector:
    """Collects all validation errors."""

    errors: list[ValidationError] = dataclasses.field(default_factory=list)

    def __call__(self, value: Any, message: Optional[str], site: ValidationSite | None):
        self.errors.append(ValidationError(value, message, site))


@dataclass(slots=True)
class TypeConstraintIn:
    """A mini-TypeConstraint so we can define constraints without having to import :TypeConstraint."""

    # numeric
    min_value: float | None = None
    max_value: float | None = None
    step_value: float | None = None
    # list-ish
    min_length: int | None = None
    max_length: int | None = None
    # string-ish
    regex: str | None = None
    starts_with: str | None = None
    ends_with: str | None = None
    # node-ish
    block_types: "list[BlockType]" = dataclasses.field(default_factory=list)
    step_types: "list[StepType]" = dataclasses.field(default_factory=list)
    file_types: "list[FileType]" = dataclasses.field(default_factory=list)
    file_formats: "list[FileFormat]" = dataclasses.field(default_factory=list)

    def into(self) -> "TypeConstraint":
        from bench.language.field import TypeConstraint

        return TypeConstraint(
            min_value=float(self.min_value) if self.min_value is not None else None,
            max_value=float(self.max_value) if self.max_value is not None else None,
            step_value=float(self.step_value) if self.step_value is not None else None,
            min_length=self.min_length,
            max_length=self.max_length,
            regex=self.regex,
            starts_with=self.starts_with,
            ends_with=self.ends_with,
            block_types=self.block_types,
            step_types=self.step_types,
            file_types=self.file_types,
            file_formats=self.file_formats,
        )


SLUG_REGEX_CHAR = r"a-z0-9-"
SLUG_REGEX = rf"^[{SLUG_REGEX_CHAR}]{{3,}}$"
EMAIL_REGEX = r"^[a-zA-Z0-9_.+\-]+@[a-zA-Z0-9\-]+\.[a-zA-Z0-9\-.]+$"
NAME_REGEX_CHAR = r"\p{L}0-9 _,;.\-'`˚ "
NAME_REGEX_INLINE = rf"[{NAME_REGEX_CHAR}]+"
NAME_REGEX = rf"^{NAME_REGEX_INLINE}$"
NAME_CONSTRAINT = TypeConstraintIn(regex=NAME_REGEX, min_length=1, max_length=128)
TITLE_CONSTRAINT = TypeConstraintIn(min_length=1, max_length=256)
SLUG_CONSTRAINT = TypeConstraintIn(regex=SLUG_REGEX)
EMAIL_CONSTRAINT = TypeConstraintIn(regex=EMAIL_REGEX)


def clean_name(name: str, sub="-") -> str:
    """Strip any invalid characters from a name."""
    return regex.sub(rf"[^{NAME_REGEX_CHAR}]", sub, name)


def constraint(
    min_value: float | None = None,
    max_value: float | None = None,
    step_value: float | None = None,
    min_length: int | None = None,
    max_length: int | None = None,
    regex: str | None = None,
    starts_with: str | None = None,
    ends_with: str | None = None,
    block_types: "list[BlockType] | None" = None,
    step_types: "list[StepType] | None" = None,
    file_types: "list[FileType] | None" = None,
    file_formats: "list[FileFormat] | None" = None,
) -> "TypeConstraintIn":
    return TypeConstraintIn(
        min_value=min_value,
        max_value=max_value,
        step_value=step_value,
        min_length=min_length,
        max_length=max_length,
        regex=regex,
        starts_with=starts_with,
        ends_with=ends_with,
        block_types=block_types if block_types is not None else [],
        step_types=step_types if step_types is not None else [],
        file_types=file_types if file_types is not None else [],
        file_formats=file_formats if file_formats is not None else [],
    )
