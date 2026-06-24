from __future__ import annotations

from destack._impl.model import ModelImpl

WORD_BITS = 64
WORD_MASK = (1 << WORD_BITS) - 1


class BitSetImpl(ModelImpl):
    """Methods for fixed-length bitsets."""

    def len(self) -> int:
        """Return the declared bit count."""

        return self.length

    def is_empty(self) -> bool:
        """Return whether the set has no declared bits."""

        return self.length == 0

    def contains(self, index: int) -> bool:
        """Return whether one bit is set."""

        check_index(self, index)

        word = index // WORD_BITS
        bit = index % WORD_BITS
        mask = 1 << bit

        return self.words[word] & mask != 0

    def count(self) -> int:
        """Count all set bits."""

        count = 0

        # count each packed word independently
        for word in self.words:
            count += (int(word) & WORD_MASK).bit_count()

        return count

    def indices(self) -> list[int]:
        """Return all set bit indices."""

        indices = []

        # scan only the declared bit range
        for index in range(self.length):
            if self.contains(index):
                indices.append(index)

        return indices


def check_index(bitset, index: int) -> None:
    """Check that an index is inside a bitset."""

    is_index = isinstance(index, int) and 0 <= index < bitset.length

    # reject non-integer or out-of-range bit indices
    if not is_index:
        raise IndexError(f"bit index out of range: {index}")
