from __future__ import annotations

class StaticKeyImpl:
    """Methods for static lookup keys."""

    def matches(self, other: StaticKeyImpl) -> bool:
        """Return whether two static keys refer to the same key."""
        ...

    def is_string_like(self) -> bool:
        """Return whether the key names a string-like member."""
        ...

    def is_number_like(self) -> bool:
        """Return whether the key names a number-like member."""
        ...

    def is_symbol_like(self) -> bool:
        """Return whether the key names a symbol-like member."""
        ...
