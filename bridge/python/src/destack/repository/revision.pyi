# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence

class Revision:
    """External revision value crossing bridge boundaries."""

    def __init__(self, id: str) -> None: ...

    """Displayed repository revision id."""
    @property
    def id(self) -> str: ...

