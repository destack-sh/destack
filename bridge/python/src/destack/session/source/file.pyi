# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence

from destack.source.module import (
    ModuleId,
)

class Change:
    """One file change observed by a session."""

    def __init__(self, path: str, uri: str, is_removed: bool, module_id: ModuleId | None) -> None: ...

    """Repository logical path."""
    @property
    def path(self) -> str: ...

    """External file URI."""
    @property
    def uri(self) -> str: ...

    """Whether the file was removed."""
    @property
    def is_removed(self) -> bool: ...

    """Updated module id when known."""
    @property
    def module_id(self) -> ModuleId | None: ...

