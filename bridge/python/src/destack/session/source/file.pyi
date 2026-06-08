# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence

from destack.source.module import (
    ModuleId,
)

class SourceFile:
    """One source file in an explicit source snapshot."""

    def __init__(self, path: str, content: SourceFileContent) -> None: ...

    """Repository-root relative path."""
    @property
    def path(self) -> str: ...

    """Full source file content."""
    @property
    def content(self) -> SourceFileContent: ...

    @staticmethod
    def text(path: str, text: str) -> SourceFile: ...

    @staticmethod
    def bytes(path: str, bytes: bytes | bytearray | Sequence[int]) -> SourceFile: ...

class SourceFileContent:
    """Full content for one source snapshot file."""

    """Text file content."""
    @staticmethod
    def text(text: str) -> SourceFileContent: ...

    """Binary file content."""
    @staticmethod
    def bytes(bytes: bytes | bytearray | Sequence[int]) -> SourceFileContent: ...

    @property
    def kind(self) -> str: ...

class FileUpdate:
    """File update projected from a source update."""

    def __init__(self, path: str, uri: str, kind: FileUpdateKind, is_removed: bool, module_id: ModuleId | None) -> None: ...

    """Repository logical path."""
    @property
    def path(self) -> str: ...

    """External file URI."""
    @property
    def uri(self) -> str: ...

    """Coarse file update kind."""
    @property
    def kind(self) -> FileUpdateKind: ...

    """Whether the file was removed."""
    @property
    def is_removed(self) -> bool: ...

    """Updated module id when known."""
    @property
    def module_id(self) -> ModuleId | None: ...

class FileUpdateKind:
    """One coarse kind for a file change."""

    """One ordinary source change."""
    @staticmethod
    def source() -> FileUpdateKind: ...

    """One `destack.json` change."""
    @staticmethod
    def config() -> FileUpdateKind: ...

    @property
    def label(self) -> str: ...

