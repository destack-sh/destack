# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence

from destack.artifact.version import (
    ArtifactVersion,
)

from destack.source.component import (
    ComponentId,
)

from destack.source.module import (
    ModuleId,
)

from destack.source.profile import (
    ProfileId,
)

class DirChecked:
    """Typed projection of one checked DIR module artifact."""

    def __init__(self, version: ArtifactVersion, module: ModuleId, profile: ProfileId, component: ComponentId, entry: ModuleId) -> None: ...

    """Exact checked facade artifact version."""
    @property
    def version(self) -> ArtifactVersion: ...

    """Checked module id."""
    @property
    def module(self) -> ModuleId: ...

    """Checked semantic profile."""
    @property
    def profile(self) -> ProfileId: ...

    """Component that owns the checked module output."""
    @property
    def component(self) -> ComponentId: ...

    """Component entry module."""
    @property
    def entry(self) -> ModuleId: ...

