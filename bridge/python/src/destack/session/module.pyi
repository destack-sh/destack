# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence

from destack.source.module import (
    ModuleId,
)

class Module:
    """One module loaded through a live session."""

    def __init__(self, id: ModuleId) -> None: ...

    """Stable source module id."""
    @property
    def id(self) -> ModuleId: ...

