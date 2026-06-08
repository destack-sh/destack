# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence

from destack.session.source.file import (
    SourceFile,
)

class SourceSnapshot:
    """Source truth used to open a live session."""

    def __init__(self, files: Sequence[SourceFile]) -> None: ...

    """Files visible to the session source root."""
    @property
    def files(self) -> list[SourceFile]: ...

