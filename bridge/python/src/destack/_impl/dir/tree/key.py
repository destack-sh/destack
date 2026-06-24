from __future__ import annotations

from destack._impl.model import ModelImpl

from typing import TYPE_CHECKING

# type imports
if TYPE_CHECKING:
    from destack._generated.core.string import StringId
    from destack._generated.dir.symbol.key import StaticKey


class NameImpl(ModelImpl):
    """Methods for property names."""

    def static_key(self) -> StaticKey:
        """Return the static key for a property name."""

        return name_static_key(self)


class KeyImpl(ModelImpl):
    """Methods for property keys."""

    def direct_static_key(self) -> StaticKey | None:
        """Return the direct static key for a non-private key."""

        if self.kind == "name":
            return name_static_key(self.name)

        return None

    def private_name(self) -> StringId | None:
        """Return the private name for a private key."""

        if self.kind == "private":
            return self.private

        return None


def name_static_key(name) -> StaticKey:
    """Return the static key for a property name."""

    from destack._generated.dir.symbol.key import StaticKeyIndex, StaticKeyName

    # numeric names map to index keys
    if name.kind == "index":
        return StaticKeyIndex(index=name.index)

    # identifiers map to named keys
    elif name.kind == "identifier":
        return StaticKeyName(name=name.identifier)
    # string names map to named keys
    else:
        return StaticKeyName(name=name.string)
