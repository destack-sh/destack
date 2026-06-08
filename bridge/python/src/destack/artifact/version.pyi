# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence

from destack.artifact.key import (
    ArtifactKey,
)

class ArtifactVersion:
    """External artifact version crossing bridge boundaries."""

    def __init__(self, key: ArtifactKey, fingerprint: str) -> None: ...

    """Semantic artifact slot."""
    @property
    def key(self) -> ArtifactKey: ...

    """Exact semantic fingerprint."""
    @property
    def fingerprint(self) -> str: ...

