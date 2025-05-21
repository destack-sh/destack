from typing import TYPE_CHECKING, Any, Callable, Collection, Optional, TypedDict, Union

import regex

from .const import BenchError

if TYPE_CHECKING:
    from bench.language import Property, TypeBase

    from .value import SomeValue

ValidationSite = Union["TypeBase", tuple["Property | Any", ...]]


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


def clean_name(name: str, sub="-") -> str:
    """Strip any invalid characters from a name."""
    return regex.sub(r"[^\p{L}0-9 _,;.\-'`˚ ]", sub, name)
