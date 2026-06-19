# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence

from destack.source.file import (
    FileId,
)

class Span:
    """Source byte span crossing bridge boundaries."""

    """File containing this span."""
    @property
    def file(self) -> FileId: ...

    """Inclusive start byte offset."""
    @property
    def start(self) -> int: ...

    """Exclusive end byte offset."""
    @property
    def end(self) -> int: ...
