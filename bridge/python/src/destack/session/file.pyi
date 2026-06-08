# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence

class SessionFile:
    """One editable file visible to a live session."""

    def __init__(self, path: str) -> None: ...

    """Repository logical path."""
    @property
    def path(self) -> str: ...

