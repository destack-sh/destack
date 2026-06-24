class DropMetadataImpl:
    """Methods for drop metadata."""

    def drop_glue(self, ty):
        """Return full drop glue for a type."""
        ...

    def drop_hook(self, ty):
        """Return the user-authored drop hook for a type."""
        ...

class DropGlueImpl:
    """Methods for drop glue."""

    def function_id(self):
        """Return the concrete function backing this glue."""
        ...

    def is_generated_function(self, target) -> bool:
        """Return whether this glue is generated for the given function."""
        ...

class DropHookImpl:
    """Methods for drop hooks."""

    def is_function(self, target) -> bool:
        """Return whether this hook calls the given function."""
        ...
