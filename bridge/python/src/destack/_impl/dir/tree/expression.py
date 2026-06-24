from __future__ import annotations

from destack._impl.model import ModelImpl

from typing import TYPE_CHECKING

# type imports
if TYPE_CHECKING:
    from destack._generated.dir.symbol.key import StaticKey


class ExpressionImpl(ModelImpl):
    """Methods for DIR expressions."""

    def static_key(self) -> StaticKey | None:
        """Return a static key for simple scalar literals."""

        if self.kind != "scalarLiteral":
            return None

        literal = self.scalar_literal

        # non-negative integer literals map to numeric static keys
        if literal.kind == "integer" and literal.integer >= 0:
            from destack._generated.dir.symbol.key import StaticKeyIndex

            return StaticKeyIndex(index=literal.integer)

        # string literals map to named static keys
        elif literal.kind == "string":
            from destack._generated.dir.symbol.key import StaticKeyName

            return StaticKeyName(name=literal.string)
        # other literals do not have static keys
        else:
            return None

    def is_reference(self) -> bool:
        """Return whether the expression directly names a value."""

        return self.kind in {"identifier", "this", "super"}
