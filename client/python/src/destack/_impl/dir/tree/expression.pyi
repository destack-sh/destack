from __future__ import annotations

from destack._generated.dir.symbol.key import StaticKey

class ExpressionImpl:
    """Methods for DIR expressions."""

    def static_key(self) -> StaticKey | None:
        """Return a static key for simple scalar literals."""
        ...

    def is_reference(self) -> bool:
        """Return whether the expression directly names a value."""
        ...
