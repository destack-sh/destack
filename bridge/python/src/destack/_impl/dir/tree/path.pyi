from __future__ import annotations

from destack._generated.core.string import StringId

class PathImpl:
    """Methods for static paths."""

    def last_segment(self) -> StringId | None:
        """Return the final path segment."""
        ...
