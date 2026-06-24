from __future__ import annotations

from destack._impl.model import ModelImpl


class TypeMetadataImpl(ModelImpl):
    """Methods for type metadata."""

    def lineage(self, ty):
        """Return lineage metadata for a type when present."""

        return self.lineage_by_type.get(ty)

    def display_name(self, ty):
        """Return the display name for a type when present."""

        return self.display_name_by_type.get(ty)

    def descriptor_global(self, ty):
        """Return the runtime type descriptor global for a type when present."""

        return self.descriptor_by_type.get(ty)
