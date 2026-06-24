from __future__ import annotations

from destack._impl.model import ModelImpl


class ProgramImpl(ModelImpl):
    """Methods for executable programs."""

    def pointer_bytes(self) -> int:
        """Return the pointer width in bytes."""

        return self.header.pointer_bytes

    def has_native(self) -> bool:
        """Return whether the program contains native execution material."""

        return self.native is not None
