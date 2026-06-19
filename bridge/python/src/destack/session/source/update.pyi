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

class TextEdit:
    """One text replacement."""

    def __init__(self, range: TextRange, text: str) -> None: ...

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

class Commit:
    """One committed edit batch."""

    """Previous revision."""
    @property
    def before(self) -> Revision: ...

    """Updated revision."""
    @property
    def after(self) -> Revision: ...

    """Changed files."""
    @property
    def changes(self) -> list[Change]: ...
