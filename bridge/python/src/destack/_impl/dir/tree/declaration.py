from __future__ import annotations

from destack._impl.model import ModelImpl

from typing import TYPE_CHECKING

# type imports
if TYPE_CHECKING:
    from destack._generated.dir.symbol.scope import ScopeKind
    from destack._generated.dir.symbol.symbol import SymbolKind, SymbolRole


class DeclarationImpl(ModelImpl):
    """Methods for declarations."""

    def is_ambient(self) -> bool:
        """Return whether the declaration is ambient."""

        # global declarations expose their own ambient flag
        if self.kind == "global":
            return self.global_.is_ambient

        # type declarations expose their own ambient flag
        elif self.kind == "type":
            return getattr(self, "type").is_ambient

        # struct declarations expose their own ambient flag
        elif self.kind == "struct":
            return self.struct.is_ambient

        # class declarations expose their own ambient flag
        elif self.kind == "class":
            return self.class_.is_ambient

        # enum declarations expose their own ambient flag
        elif self.kind == "enum":
            return self.enum.is_ambient

        # interface declarations expose their own ambient flag
        elif self.kind == "interface":
            return self.interface.is_ambient

        # extension declarations expose their own ambient flag
        elif self.kind == "extension":
            return self.extension.is_ambient

        # function declarations expose their own ambient flag
        elif self.kind == "function":
            return self.function.is_ambient
        # module and error declarations are not ambient
        else:
            return False

    def is_abstract(self) -> bool:
        """Return whether the declaration is abstract."""

        # class declarations store abstractness directly
        if self.kind == "class":
            return self.class_.is_abstract

        # functions store abstractness on their signature
        elif self.kind == "function":
            return self.function.signature.is_abstract
        # all other declaration forms are concrete
        else:
            return False

    def symbol_kind(self) -> SymbolKind | None:
        """Return the declaration symbol kind."""

        # global and module declarations do not introduce regular symbols
        if self.kind in {"global", "module"}:
            return None

        # nominal aliases use a distinct symbol kind
        elif self.kind == "type":
            return "newtype" if getattr(self, "type").is_nominal else "typeAlias"

        # nominal interfaces use a distinct symbol kind
        elif self.kind == "interface":
            return "newtypeInterface" if self.interface.is_nominal else "interface"

        # functions use the stable function symbol kind
        elif self.kind == "function":
            return "function"
        # other declaration variants map directly to symbol kinds
        else:
            return self.kind

    def symbol_role(self) -> SymbolRole | None:
        """Return the declaration symbol role."""

        # global and module declarations do not introduce regular symbols
        if self.kind in {"global", "module"}:
            return None

        # functions and type aliases are leaf items
        elif self.kind in {"function", "type"}:
            return "item"
        # aggregate declaration forms own namespace members
        else:
            return "namespace"

    def symbol_scope_kind(self) -> ScopeKind | None:
        """Return the scope kind opened by the declaration."""

        # functions open function scopes
        if self.kind == "function":
            return "function"

        # namespace declarations open namespace scopes
        elif self.symbol_role() == "namespace":
            return "namespace"
        # leaf declarations do not open scopes
        else:
            return None
