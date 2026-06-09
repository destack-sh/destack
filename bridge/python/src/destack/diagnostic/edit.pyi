# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence

from destack.source.file import (
    FileId,
)

from destack.source.span import (
    Span,
)

class Replacement:
    """One source replacement crossing bridge boundaries."""

    def __init__(self, span: Span, new_text: str) -> None: ...

    """Source span to replace."""
    @property
    def span(self) -> Span: ...

    """Replacement text."""
    @property
    def new_text(self) -> str: ...

class FilePatch:
    """Edits for a single file."""

    def __init__(self, file: FileId, replacements: Sequence[Replacement]) -> None: ...

    """Edited file."""
    @property
    def file(self) -> FileId: ...

    """Source replacements."""
    @property
    def replacements(self) -> list[Replacement]: ...

class BatchEdit:
    """Edits across multiple files."""

    def __init__(self, files: Sequence[FilePatch]) -> None: ...

    """Per-file edits."""
    @property
    def files(self) -> list[FilePatch]: ...

