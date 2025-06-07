from typing import TYPE_CHECKING, Any, Optional

from ..builtin import BenchError

if TYPE_CHECKING:
    pass


class ValidationError(BenchError, ValueError):
    def __init__(
        self,
        value: Any,
        message: Optional[str],
    ):
        if value is not None:
            super().__init__(f"{value!r}: {message}")
        else:
            super().__init__(f"{message}")
        self.value = value
        self.message = message
