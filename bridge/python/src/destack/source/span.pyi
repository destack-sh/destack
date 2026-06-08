# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence

from destack.source.file import (
    FileId,
)

class Span:
    """Source byte span crossing bridge boundaries."""

    def __init__(self, file: FileId, start: int, end: int) -> None: ...

    """File containing this span."""
    @property
    def file(self) -> FileId: ...

    """Inclusive start byte offset."""
    @property
    def start(self) -> int: ...

    """Exclusive end byte offset."""
    @property
    def end(self) -> int: ...

class LabeledSpan:
    """Source span with a display label."""

    def __init__(self, span: Span, label: str) -> None: ...

    """Source span."""
    @property
    def span(self) -> Span: ...

    """Display label."""
    @property
    def label(self) -> str: ...

