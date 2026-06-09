# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence

from destack.artifact.version import (
    ArtifactVersion,
)

from destack.source.module import (
    ModuleId,
)

class DirParsed:
    """Typed projection of one parsed DIR artifact."""

    def __init__(self, version: ArtifactVersion, module: ModuleId) -> None: ...

    """Exact parsed artifact version."""
    @property
    def version(self) -> ArtifactVersion: ...

    """Parsed module id."""
    @property
    def module(self) -> ModuleId: ...

