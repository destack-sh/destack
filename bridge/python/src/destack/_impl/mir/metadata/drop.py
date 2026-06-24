from __future__ import annotations

from destack._impl.model import ModelImpl


class DropMetadataImpl(ModelImpl):
    """Methods for drop metadata."""

    def drop_glue(self, ty):
        """Return full drop glue for a type."""

        return self.glue_by_type.get(ty)

    def drop_hook(self, ty):
        """Return the user-authored drop hook for a type."""

        return self.hooks_by_type.get(ty)


class DropGlueImpl(ModelImpl):
    """Methods for drop glue."""

    def function_id(self):
        """Return the concrete function backing this glue."""

        if self.kind == "generated":
            return self.function

        return None

    def is_generated_function(self, target) -> bool:
        """Return whether this glue is generated for the given function."""

        if self.kind != "generated":
            return False

        return self.function == target


class DropHookImpl(ModelImpl):
    """Methods for drop hooks."""

    def is_function(self, target) -> bool:
        """Return whether this hook calls the given function."""

        return self.function == target
