from typing import TYPE_CHECKING, Any, Optional, Union

from ..builtin import BenchError

if TYPE_CHECKING:
    from bench.language import Property, TypeBase


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
