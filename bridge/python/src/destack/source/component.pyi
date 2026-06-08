# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence

class ComponentId:
    """External component id crossing bridge boundaries."""

    def __init__(self, id: str) -> None: ...

    """Canonical lowercase hex component id."""
    @property
    def id(self) -> str: ...

