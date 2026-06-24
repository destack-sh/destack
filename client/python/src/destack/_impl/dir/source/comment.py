from __future__ import annotations

from destack._impl.model import ModelImpl

LEADING = 1 << 0
TRAILING = 1 << 1


class CommentNewlinesImpl(ModelImpl):
    """Methods for comment newline flags."""

    def has_leading_newline(self) -> bool:
        """Return whether a comment has a leading newline."""

        return self.bits & LEADING != 0

    def has_trailing_newline(self) -> bool:
        """Return whether a comment has a trailing newline."""

        return self.bits & TRAILING != 0
