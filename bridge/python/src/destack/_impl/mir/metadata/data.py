from __future__ import annotations

from destack._impl.model import ModelImpl


class DataLayoutImpl(ModelImpl):
    """Methods for target data layout."""

    def pointer_bits(self) -> int:
        """Return the pointer width in bits."""

        return self.pointer_bytes * 8
