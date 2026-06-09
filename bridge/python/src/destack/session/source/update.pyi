# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence

from destack.repository.revision import (
    Revision,
)

from destack.session.source.file import (
    Change,
)

class TextRange:
    """One text range in byte offsets."""

    def __init__(self, start: int, end: int) -> None: ...

    """Inclusive start byte offset."""
    @property
    def start(self) -> int: ...

    """Exclusive end byte offset."""
    @property
    def end(self) -> int: ...

class TextEdit:
    """One text replacement."""

    def __init__(self, range: TextRange, text: str) -> None: ...

    """Replaced byte range."""
    @property
    def range(self) -> TextRange: ...

    """Replacement text."""
    @property
    def text(self) -> str: ...

class Edit:
    """One edit accepted by a session."""

    """Replace or create one text file."""
    @staticmethod
    def set_text(path: str, text: str) -> Edit: ...

    """Apply text replacements to one tracked text file."""
    @staticmethod
    def edit_text(path: str, edits: Sequence[TextEdit]) -> Edit: ...

    """Replace or create one binary file."""
    @staticmethod
    def set_bytes(path: str, bytes: bytes | bytearray | Sequence[int]) -> Edit: ...

    """Remove one file."""
    @staticmethod
    def remove(path: str) -> Edit: ...

    """Move one file."""
    @staticmethod
    def move_file(from_: str, to: str) -> Edit: ...

    @property
    def kind(self) -> str: ...

    @property
    def bytes(self) -> list[int] | None: ...

    @property
    def edits(self) -> list[TextEdit] | None: ...

    @property
    def from(self) -> str | None: ...

    @property
    def path(self) -> str | None: ...

    @property
    def text(self) -> str | None: ...

    @property
    def to(self) -> str | None: ...

class Commit:
    """One committed edit batch."""

    def __init__(self, before: Revision, after: Revision, changes: Sequence[Change]) -> None: ...

    """Previous revision."""
    @property
    def before(self) -> Revision: ...

    """Updated revision."""
    @property
    def after(self) -> Revision: ...

    """Changed files."""
    @property
    def changes(self) -> list[Change]: ...

