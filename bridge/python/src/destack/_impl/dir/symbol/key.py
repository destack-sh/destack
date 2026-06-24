from __future__ import annotations

from destack._impl.model import ModelImpl


class StaticKeyImpl(ModelImpl):
    """Methods for static lookup keys."""

    def matches(self, other: StaticKeyImpl) -> bool:
        """Return whether two static keys refer to the same key."""

        if self.kind != other.kind:
            return False

        # compare string keys by generated value equality
        if self.kind == "name":
            return self.name == other.name
        # compare index keys by numeric index
        elif self.kind == "index":
            return self.index == other.index
        # compare symbol keys by generated value equality
        else:
            return self.symbol == other.symbol

    def is_string_like(self) -> bool:
        """Return whether the key names a string-like member."""

        return self.kind == "name"

    def is_number_like(self) -> bool:
        """Return whether the key names a number-like member."""

        return self.kind == "index"

    def is_symbol_like(self) -> bool:
        """Return whether the key names a symbol-like member."""

        return self.kind == "symbol"
