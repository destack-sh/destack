# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence

from destack.session.source.update import (
    FileEdit,
)

class Source:
    """Source input used to open a live session."""

    """Filesystem source rooted at a path."""
    @staticmethod
    def file_system(path: str) -> Source: ...

    """In-memory filesystem source seeded by file edits."""
    @staticmethod
    def memory(root: str, edits: Sequence[FileEdit]) -> Source: ...

    @property
    def kind(self) -> str: ...

    @property
    def edits(self) -> list[FileEdit] | None: ...

    @property
    def path(self) -> str | None: ...

    @property
    def root(self) -> str | None: ...

