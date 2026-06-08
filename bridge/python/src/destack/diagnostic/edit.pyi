# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence

from destack.source.file import (
    FileId,
)

from destack.source.span import (
    Span,
)

class Edit:
    """One source edit crossing bridge boundaries."""

    def __init__(self, span: Span, new_text: str) -> None: ...

    """Source span to replace."""
    @property
    def span(self) -> Span: ...

    """Replacement text."""
    @property
    def new_text(self) -> str: ...

class FileEdit:
    """Edits for a single file."""

    def __init__(self, file: FileId, edits: Sequence[Edit]) -> None: ...

    """Edited file."""
    @property
    def file(self) -> FileId: ...

    """Source edits."""
    @property
    def edits(self) -> list[Edit]: ...

class BatchEdit:
    """Edits across multiple files."""

    def __init__(self, files: Sequence[FileEdit]) -> None: ...

    """Per-file edits."""
    @property
    def files(self) -> list[FileEdit]: ...

