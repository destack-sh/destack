from __future__ import annotations

from destack._impl.model import ModelImpl


class ValueSliceImpl(ModelImpl):
    """Methods for MIR value slices."""

    def is_empty(self) -> bool:
        """Return whether the slice has no values."""

        return self.count == 0

    def len(self) -> int:
        """Return the number of values in the slice."""

        return self.count
