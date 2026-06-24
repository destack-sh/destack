from __future__ import annotations

class ProgramImpl:
    """Methods for executable programs."""

    def pointer_bytes(self) -> int:
        """Return the pointer width in bytes."""
        ...

    def has_native(self) -> bool:
        """Return whether the program contains native execution material."""
        ...
