from __future__ import annotations

from destack._impl.model import ModelImpl

from typing import TYPE_CHECKING

# type imports
if TYPE_CHECKING:
    from destack._generated.dir.symbol.key import StaticKey
    from destack._generated.dir.symbol.scope import ScopeKind
    from destack._generated.dir.symbol.symbol import SymbolKind, SymbolSpace
    from destack._generated.dir.tree.property import MemberSlot


class PropertyImpl(ModelImpl):
    """Methods for object-like properties."""

    def has_static_modifier(self) -> bool:
        """Return whether a property has a static modifier."""

        return False

    def has_implicit_receiver(self) -> bool:
        """Return whether a property receives an implicit receiver."""

        return self.kind == "method"

    def is_instance_member(self) -> bool:
        """Return whether a property belongs to an instance shape."""

        return self.kind in {"field", "method", "spread"}


class MemberImpl(ModelImpl):
    """Methods for declaration members."""

    def slot(self) -> MemberSlot | None:
        """Return the slot occupied by a declaration member."""

        from destack._generated.dir.symbol.key import StaticKeyName
        from destack._generated.dir.tree.property import (
            MemberSlotCall,
            MemberSlotConstructor,
            MemberSlotKey,
            MemberSlotNew,
        )

        # associated members use their declared name as the member slot
        if self.kind in {"associatedType", "associatedConst"}:
            return MemberSlotKey(key=StaticKeyName(name=self.name))

        # fields use direct static keys when available
        elif self.kind == "field":
            key = self.key.direct_static_key()

            return None if key is None else MemberSlotKey(key=key)

        # keyed methods use direct static keys when available
        elif self.kind == "method" and self.key is not None:
            key = self.key.direct_static_key()

            return None if key is None else MemberSlotKey(key=key)

        # constructors occupy their own well-known slot
        elif self.kind == "method" and self.signature.role == "constructor":
            return MemberSlotConstructor()

        # new signatures occupy their own well-known slot
        elif self.kind == "method" and self.signature.role == "new":
            return MemberSlotNew()

        # call signatures occupy their own well-known slot
        elif self.kind == "method" and self.signature.role == "call":
            return MemberSlotCall()
        # other member forms do not occupy named slots
        else:
            return None

    def symbol_key(self) -> StaticKey | None:
        """Return the symbol key declared by a member."""

        slot = self.slot()

        return slot.key if slot is not None and slot.kind == "key" else None

    def symbol_kind(self) -> SymbolKind | None:
        """Return the symbol kind declared by a member."""

        # associated types declare associated type symbols
        if self.kind == "associatedType":
            return "associatedType"

        # associated constants declare associated constant symbols
        elif self.kind == "associatedConst":
            return "associatedConst"

        # fields declare variable symbols
        elif self.kind == "field":
            return "variable"

        # keyed methods declare function symbols
        elif self.kind == "method" and self.key is not None:
            return "function"
        # other members do not declare symbols
        else:
            return None

    def symbol_space(self) -> SymbolSpace | None:
        """Return the symbol space declared by a member."""

        kind = self.symbol_kind()

        # associated types bind in type space
        if kind == "associatedType":
            return "type"

        # values bind in value space
        elif kind in {"associatedConst", "variable", "function"}:
            return "value"
        # members without symbols do not bind in a symbol space
        else:
            return None

    def symbol_scope_kind(self) -> ScopeKind | None:
        """Return the scope kind opened by a member."""

        # associated types open type scopes
        if self.kind == "associatedType":
            return "type"

        # methods open function scopes
        elif self.kind == "method":
            return "function"
        # other members do not open scopes
        else:
            return None

    def has_static_modifier(self) -> bool:
        """Return whether a member has a static modifier."""

        # field and method members store staticness directly
        if self.kind in {"field", "method"}:
            return self.is_static

        elif self.kind == "staticBlock":
            return True
        # other members are not static
        else:
            return False

    def has_implicit_receiver(self) -> bool:
        """Return whether a member receives an implicit receiver."""

        return self.kind in {"field", "method"}

    def is_instance_member(self) -> bool:
        """Return whether a member belongs to an instance shape."""

        return self.has_implicit_receiver() and not self.has_static_modifier()
