from __future__ import annotations

from destack._generated.dir.symbol.key import StaticKey
from destack._generated.dir.symbol.scope import ScopeKind
from destack._generated.dir.symbol.symbol import SymbolKind, SymbolSpace
from destack._generated.dir.tree.property import MemberSlot

class PropertyImpl:
    """Methods for object-like properties."""

    def has_static_modifier(self) -> bool:
        """Return whether a property has a static modifier."""
        ...

    def has_implicit_receiver(self) -> bool:
        """Return whether a property receives an implicit receiver."""
        ...

    def is_instance_member(self) -> bool:
        """Return whether a property belongs to an instance shape."""
        ...

class MemberImpl:
    """Methods for declaration members."""

    def slot(self) -> MemberSlot | None:
        """Return the slot occupied by a declaration member."""
        ...

    def symbol_key(self) -> StaticKey | None:
        """Return the symbol key declared by a member."""
        ...

    def symbol_kind(self) -> SymbolKind | None:
        """Return the symbol kind declared by a member."""
        ...

    def symbol_space(self) -> SymbolSpace | None:
        """Return the symbol space declared by a member."""
        ...

    def symbol_scope_kind(self) -> ScopeKind | None:
        """Return the scope kind opened by a member."""
        ...

    def has_static_modifier(self) -> bool:
        """Return whether a member has a static modifier."""
        ...

    def has_implicit_receiver(self) -> bool:
        """Return whether a member receives an implicit receiver."""
        ...

    def is_instance_member(self) -> bool:
        """Return whether a member belongs to an instance shape."""
        ...
