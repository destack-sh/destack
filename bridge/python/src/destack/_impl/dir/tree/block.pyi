from __future__ import annotations

from destack._generated.dir.tree.node import LocalNodeId

class BlockImpl:
    """Methods for DIR blocks."""

    def is_explicit(self) -> bool:
        """Return whether the block has explicit block syntax."""
        ...

    def is_empty(self) -> bool:
        """Return whether the block has no expressions."""
        ...

    def len(self) -> int:
        """Return the number of expressions in the block."""
        ...

    def first_expression(self) -> LocalNodeId | None:
        """Return the first expression in evaluation order."""
        ...

    def last_expression(self) -> LocalNodeId | None:
        """Return the last expression in evaluation order."""
        ...
