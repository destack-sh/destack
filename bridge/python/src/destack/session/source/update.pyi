# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence

from destack.repository.revision import (
    Revision,
)

from destack.session.source.file import (
    FileUpdate,
)

class TextRange:
    """Source text range in byte offsets."""

    def __init__(self, start: int, end: int) -> None: ...

    """Inclusive start byte offset."""
    @property
    def start(self) -> int: ...

    """Exclusive end byte offset."""
    @property
    def end(self) -> int: ...

class TextEdit:
    """Source text replacement."""

    def __init__(self, range: TextRange, text: str) -> None: ...

    """Replaced byte range."""
    @property
    def range(self) -> TextRange: ...

    """Replacement text."""
    @property
    def text(self) -> str: ...

class SourceEdit:
    """One source edit accepted by a session update."""

    """Replace or create one text file."""
    @staticmethod
    def set_text(path: str, text: str) -> SourceEdit: ...

    """Apply text replacements to one tracked text file."""
    @staticmethod
    def edit_text(path: str, edits: Sequence[TextEdit]) -> SourceEdit: ...

    """Replace or create one binary file."""
    @staticmethod
    def set_bytes(path: str, bytes: bytes | bytearray | Sequence[int]) -> SourceEdit: ...

    """Remove one file."""
    @staticmethod
    def remove(path: str) -> SourceEdit: ...

    """Move one file."""
    @staticmethod
    def move_file(from_: str, to: str) -> SourceEdit: ...

    @property
    def kind(self) -> str: ...

class SourceUpdate:
    """Source update applied through one session ref."""

    def __init__(self, base: Revision | None, edits: Sequence[SourceEdit]) -> None: ...

    """Expected base revision."""
    @property
    def base(self) -> Revision | None: ...

    """Source edits in this atomic update."""
    @property
    def edits(self) -> list[SourceEdit]: ...

class SourceUpdateResult:
    """Source update result."""

    def __init__(self, before: Revision, after: Revision, files: Sequence[FileUpdate]) -> None: ...

    """Previous revision."""
    @property
    def before(self) -> Revision: ...

    """Updated revision."""
    @property
    def after(self) -> Revision: ...

    """Changed files."""
    @property
    def files(self) -> list[FileUpdate]: ...

