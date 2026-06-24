from __future__ import annotations

from destack._impl.model import ModelImpl

from typing import TYPE_CHECKING

# type imports
if TYPE_CHECKING:
    from destack._generated.dir.tree.node import LocalNodeId


class BlockImpl(ModelImpl):
    """Methods for DIR blocks."""

    def is_explicit(self) -> bool:
        """Return whether the block has explicit block syntax."""

        return self.form in {"explicit", "do"}

    def is_empty(self) -> bool:
        """Return whether the block has no expressions."""

        return len(self.leading_expressions) == 0 and self.tail_expression is None

    def len(self) -> int:
        """Return the number of expressions in the block."""

        return len(self.leading_expressions) + int(self.tail_expression is not None)

    def first_expression(self) -> LocalNodeId | None:
        """Return the first expression in evaluation order."""

        # leading expressions are evaluated first
        if self.leading_expressions:
            return self.leading_expressions[0]

        return self.tail_expression

    def last_expression(self) -> LocalNodeId | None:
        """Return the last expression in evaluation order."""

        # tail expressions are evaluated last
        if self.tail_expression is not None:
            return self.tail_expression

        # otherwise the last leading expression is last
        if self.leading_expressions:
            return self.leading_expressions[-1]

        return None
