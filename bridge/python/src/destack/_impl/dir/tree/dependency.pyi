from __future__ import annotations

from destack._generated.core.string import StringId
from destack._generated.dir.symbol.export import ExportSelector
from destack._generated.dir.symbol.key import StaticKey
from destack._generated.dir.symbol.symbol import SymbolKind

class DependencyItemImpl:
    """Methods for dependency import and export items."""

    def local_string_key(self) -> StringId | None:
        """Return the local key introduced by an import binding."""
        ...

    def local_import_alias_name(self) -> StringId | None:
        """Return the local alias name for an import binding."""
        ...

    def default_import_alias_name(self) -> StringId | None:
        """Return the alias name for a default import binding."""
        ...

    def symbol_key(self) -> StaticKey | None:
        """Return the symbol key introduced by an import binding."""
        ...

    def symbol_kind(self) -> SymbolKind | None:
        """Return the symbol kind introduced by an import binding."""
        ...

    def export_source_key(self) -> StaticKey | None:
        """Return the exported source key."""
        ...

    def export_selector(self) -> ExportSelector | None:
        """Return the export selector represented by the item."""
        ...

    def is_star_export(self) -> bool:
        """Return whether the item exports all names from another module."""
        ...

    def is_default_value_export(self) -> bool:
        """Return whether the item exports the imported default value."""
        ...
