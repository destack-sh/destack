from __future__ import annotations

from destack._generated.core.string import StringId
from destack._generated.dir.symbol.key import StaticKey

class NameImpl:
    """Methods for property names."""

    def static_key(self) -> StaticKey:
        """Return the static key for a property name."""
        ...

class KeyImpl:
    """Methods for property keys."""

    def direct_static_key(self) -> StaticKey | None:
        """Return the direct static key for a non-private key."""
        ...

    def private_name(self) -> StringId | None:
        """Return the private name for a private key."""
        ...
