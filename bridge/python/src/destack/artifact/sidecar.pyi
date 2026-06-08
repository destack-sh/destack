# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence

from destack.source.file import (
    FileContent,
)

class ArtifactSidecarLabel:
    """One stable sidecar label crossing bridge boundaries."""

    def __init__(self, key: str, value: str) -> None: ...

    """Label key."""
    @property
    def key(self) -> str: ...

    """Label value."""
    @property
    def value(self) -> str: ...

class ArtifactSidecar:
    """One named artifact sidecar crossing bridge boundaries."""

    def __init__(self, name: str, labels: Sequence[ArtifactSidecarLabel], content: FileContent) -> None: ...

    """Sidecar name."""
    @property
    def name(self) -> str: ...

    """Stable labels describing this sidecar."""
    @property
    def labels(self) -> list[ArtifactSidecarLabel]: ...

    """Sidecar content."""
    @property
    def content(self) -> FileContent: ...

