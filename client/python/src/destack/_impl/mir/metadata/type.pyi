class TypeMetadataImpl:
    """Methods for type metadata."""

    def lineage(self, ty):
        """Return lineage metadata for a type when present."""
        ...

    def display_name(self, ty):
        """Return the display name for a type when present."""
        ...

    def descriptor_global(self, ty):
        """Return the runtime type descriptor global for a type when present."""
        ...
