from __future__ import annotations

from destack._impl.model import ModelImpl

from typing import TYPE_CHECKING

# type imports
if TYPE_CHECKING:
    from destack._generated.core.string import StringId


class PathImpl(ModelImpl):
    """Methods for static paths."""

    def last_segment(self) -> StringId | None:
        """Return the final path segment."""

        if not self.segments:
            return None

        return self.segments[-1]
