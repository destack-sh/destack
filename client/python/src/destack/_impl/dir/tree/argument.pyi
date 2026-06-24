from __future__ import annotations

from destack._generated.dir.symbol.key import StaticKey
from destack._generated.dir.symbol.symbol import SymbolKind, SymbolSpace
from destack._generated.dir.tree.node import LocalNodeId

class GenericParameterImpl:
    """Methods for generic parameters."""

    def symbol_key(self) -> StaticKey | None:
        """Return the key declared by the parameter."""
        ...

    def symbol_space(self) -> SymbolSpace | None:
        """Return the symbol space declared by the parameter."""
        ...

    def symbol_kind(self) -> SymbolKind | None:
        """Return the symbol kind declared by the parameter."""
        ...

class ParameterImpl:
    """Methods for callable parameters."""

    def symbol_key(self) -> StaticKey | None:
        """Return the key declared by the parameter."""
        ...

    def default_value(self) -> LocalNodeId | None:
        """Return the parameter default value node."""
        ...
