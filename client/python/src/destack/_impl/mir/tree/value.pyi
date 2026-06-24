from __future__ import annotations

class ValueSliceImpl:
    """Methods for MIR value slices."""

    def is_empty(self) -> bool:
        """Return whether the slice has no values."""
        ...

    def len(self) -> int:
        """Return the number of values in the slice."""
        ...
