class DispatchMetadataImpl:
    """Methods for dispatch metadata."""

    def vtable(self, ty):
        """Return the vtable for a type when present."""
        ...

    def dynamic_table(self, concrete, constraint):
        """Return the dynamic table for a concrete type and constraint."""
        ...

    def dynamic_shape(self, constraint):
        """Return dynamic shape metadata for a constraint type id."""
        ...
