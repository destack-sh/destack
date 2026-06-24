from __future__ import annotations

from destack._generated.dir.symbol.scope import ScopeKind
from destack._generated.dir.symbol.symbol import SymbolKind, SymbolRole

class DeclarationImpl:
    """Methods for declarations."""

    def is_ambient(self) -> bool:
        """Return whether the declaration is ambient."""
        ...

    def is_abstract(self) -> bool:
        """Return whether the declaration is abstract."""
        ...

    def symbol_kind(self) -> SymbolKind | None:
        """Return the declaration symbol kind."""
        ...

    def symbol_role(self) -> SymbolRole | None:
        """Return the declaration symbol role."""
        ...

    def symbol_scope_kind(self) -> ScopeKind | None:
        """Return the scope kind opened by the declaration."""
        ...
