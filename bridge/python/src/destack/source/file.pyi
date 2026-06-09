# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence

class FileId:
    """External file id crossing bridge boundaries."""

    def __init__(self, id: str) -> None: ...

    """Canonical lowercase hex file id."""
    @property
    def id(self) -> str: ...

class FileContentId:
    """External file content id crossing bridge boundaries."""

    def __init__(self, id: str) -> None: ...

    """Canonical lowercase hex file content id."""
    @property
    def id(self) -> str: ...

class FileContent:
    """Full file content crossing bridge boundaries."""

    """Text file content."""
    @staticmethod
    def text(content: str) -> FileContent: ...

    """Binary file content."""
    @staticmethod
    def binary(content: bytes | bytearray | Sequence[int]) -> FileContent: ...

    @property
    def kind(self) -> str: ...

    @property
    def binary_content(self) -> list[int] | None: ...

    @property
    def text_content(self) -> str | None: ...

