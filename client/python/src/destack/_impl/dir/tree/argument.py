from __future__ import annotations

from destack._impl.model import ModelImpl

from typing import TYPE_CHECKING

# type imports
if TYPE_CHECKING:
    from destack._generated.dir.symbol.key import StaticKey
    from destack._generated.dir.symbol.symbol import SymbolKind, SymbolSpace
    from destack._generated.dir.tree.node import LocalNodeId


class GenericParameterImpl(ModelImpl):
    """Methods for generic parameters."""

    def symbol_key(self) -> StaticKey | None:
        """Return the key declared by the parameter."""

        # error parameters do not declare names
        if self.kind == "error":
            return None

        from destack._generated.dir.symbol.key import StaticKeyName

        return StaticKeyName(name=self.name)

    def symbol_space(self) -> SymbolSpace | None:
        """Return the symbol space declared by the parameter."""

        # type parameters bind in type space
        if self.kind in {"type", "variadicType"}:
            return "type"

        # value parameters bind in value space
        elif self.kind in {"value", "variadicValue"}:
            return "value"
        # error parameters do not bind in a symbol space
        else:
            return None

    def symbol_kind(self) -> SymbolKind | None:
        """Return the symbol kind declared by the parameter."""

        # type parameters declare generic type symbols
        if self.kind in {"type", "variadicType"}:
            return "genericTypeParameter"

        # value parameters declare generic value symbols
        elif self.kind in {"value", "variadicValue"}:
            return "genericValueParameter"
        # error parameters do not declare symbols
        else:
            return None


class ParameterImpl(ModelImpl):
    """Methods for callable parameters."""

    def symbol_key(self) -> StaticKey | None:
        """Return the key declared by the parameter."""

        # only named parameter forms bind a parameter name
        if self.kind in {"named", "variadicNamed"}:
            from destack._generated.dir.symbol.key import StaticKeyName

            return StaticKeyName(name=self.name)

        return None

    def default_value(self) -> LocalNodeId | None:
        """Return the parameter default value node."""

        # only concrete parameter forms can carry defaults
        if self.kind in {"named", "pattern"}:
            return self.default

        return None
