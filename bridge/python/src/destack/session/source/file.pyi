# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence

from destack.source.module import (
    ModuleId,
)

class FileChange:
    """Observed file change projected from a file update."""

    def __init__(self, path: str, uri: str, kind: FileChangeKind, is_removed: bool, module_id: ModuleId | None) -> None: ...

    """Repository logical path."""
    @property
    def path(self) -> str: ...

    """External file URI."""
    @property
    def uri(self) -> str: ...

    """Coarse file change kind."""
    @property
    def kind(self) -> FileChangeKind: ...

    """Whether the file was removed."""
    @property
    def is_removed(self) -> bool: ...

    """Updated module id when known."""
    @property
    def module_id(self) -> ModuleId | None: ...

class FileChangeKind:
    """One coarse kind for a file change."""

    """One ordinary source change."""
    @staticmethod
    def source() -> FileChangeKind: ...

    """One `destack.json` change."""
    @staticmethod
    def config() -> FileChangeKind: ...

    @property
    def label(self) -> str: ...

