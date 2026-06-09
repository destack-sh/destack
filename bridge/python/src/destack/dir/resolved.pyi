# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence

from destack.artifact.version import (
    ArtifactVersion,
)

from destack.source.module import (
    ModuleId,
)

from destack.source.profile import (
    ProfileId,
)

class DirResolved:
    """Typed projection of one resolved DIR artifact."""

    def __init__(self, version: ArtifactVersion, module: ModuleId, profile: ProfileId) -> None: ...

    """Exact resolved artifact version."""
    @property
    def version(self) -> ArtifactVersion: ...

    """Resolved module id."""
    @property
    def module(self) -> ModuleId: ...

    """Resolved semantic profile."""
    @property
    def profile(self) -> ProfileId: ...

