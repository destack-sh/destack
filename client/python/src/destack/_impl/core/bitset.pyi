class BitSetImpl:
    """Methods for fixed-length bitsets."""

    def len(self) -> int:
        """Return the declared bit count."""
        ...

    def is_empty(self) -> bool:
        """Return whether the set has no declared bits."""
        ...

    def contains(self, index: int) -> bool:
        """Return whether one bit is set."""
        ...

    def count(self) -> int:
        """Count all set bits."""
        ...

    def indices(self) -> list[int]:
        """Return all set bit indices."""
        ...
