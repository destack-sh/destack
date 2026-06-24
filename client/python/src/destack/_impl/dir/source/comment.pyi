class CommentNewlinesImpl:
    """Methods for comment newline flags."""

    def has_leading_newline(self) -> bool:
        """Return whether a comment has a leading newline."""
        ...

    def has_trailing_newline(self) -> bool:
        """Return whether a comment has a trailing newline."""
        ...
