from __future__ import annotations

from destack._impl.model import ModelImpl

from typing import TYPE_CHECKING

# type imports
if TYPE_CHECKING:
    from destack._generated.core.string import StringId
    from destack._generated.dir.symbol.export import ExportSelector
    from destack._generated.dir.symbol.key import StaticKey
    from destack._generated.dir.symbol.symbol import SymbolKind


class DependencyItemImpl(ModelImpl):
    """Methods for dependency import and export items."""

    def local_string_key(self) -> StringId | None:
        """Return the local key introduced by an import binding."""

        # non-binding items do not introduce local names
        if self.kind != "binding":
            return None

        return self.alias or name_string(self.name)

    def local_import_alias_name(self) -> StringId | None:
        """Return the local alias name for an import binding."""

        # non-binding items do not introduce local names
        if self.kind != "binding":
            return None

        # default imports may use their imported name as the local alias
        if self.binding == "default":
            return name_string(self.name) or self.alias

        return self.alias

    def default_import_alias_name(self) -> StringId | None:
        """Return the alias name for a default import binding."""

        # only default bindings have default import aliases
        if self.kind != "binding" or self.binding != "default":
            return None

        return name_string(self.name)

    def symbol_key(self) -> StaticKey | None:
        """Return the symbol key introduced by an import binding."""

        # non-binding items do not introduce local names
        if self.kind != "binding":
            return None

        # explicit aliases become the local symbol key
        if self.alias is not None:
            return static_name(self.alias)

        return name_static_key(self.name)

    def symbol_kind(self) -> SymbolKind | None:
        """Return the symbol kind introduced by an import binding."""

        if self.kind == "binding":
            return "import"

        return None

    def export_source_key(self) -> StaticKey | None:
        """Return the exported source key."""

        if self.kind == "binding":
            return name_static_key(self.name)

        return None

    def export_selector(self) -> ExportSelector | None:
        """Return the export selector represented by the item."""

        # non-binding items do not export a selected binding
        if self.kind != "binding":
            return None

        # default bindings export the default selector
        if self.binding == "default":
            from destack._generated.dir.symbol.export import ExportSelectorDefault

            return ExportSelectorDefault()

        # namespace bindings export the namespace selector
        elif self.binding == "namespace":
            from destack._generated.dir.symbol.export import ExportSelectorNamespace

            return ExportSelectorNamespace()
        # named bindings export their named selector if one exists
        else:
            key = name_static_key(self.name)

            # missing names do not export named selectors
            if key is None:
                return None

            from destack._generated.dir.symbol.export import ExportSelectorNamed

            return ExportSelectorNamed(named=key)

    def is_star_export(self) -> bool:
        """Return whether the item exports all names from another module."""

        return (
            self.kind == "binding"
            and self.binding == "namespace"
            and self.alias is None
        )

    def is_default_value_export(self) -> bool:
        """Return whether the item exports the imported default value."""

        return (
            self.kind == "binding"
            and self.binding == "default"
            and self.value is not None
        )


def name_string(name) -> StringId | None:
    """Return the string value of one dependency name."""

    # missing and numeric names do not have string values
    if name is None or name.kind == "index":
        return None

    # identifiers store their string in the identifier field
    elif name.kind == "identifier":
        return name.identifier
    # string names store their string in the string field
    else:
        return name.string


def name_static_key(name) -> StaticKey | None:
    """Return the static key value of one dependency name."""

    # missing names do not have static keys
    if name is None:
        return None

    # numeric names map to index keys
    elif name.kind == "index":
        from destack._generated.dir.symbol.key import StaticKeyIndex

        return StaticKeyIndex(index=name.index)
    # string-like names map to name keys
    else:
        string = name_string(name)
        if string is None:
            return None

        return static_name(string)


def static_name(name: StringId) -> StaticKey:
    """Return one name static key."""

    from destack._generated.dir.symbol.key import StaticKeyName

    return StaticKeyName(name=name)
