# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence

from destack.artifact.dependency import (
    ArtifactDependency,
)

from destack.artifact.sidecar import (
    ArtifactSidecar,
)

from destack.artifact.version import (
    ArtifactVersion,
)

from destack.diagnostic.diagnostic import (
    Diagnostic,
)

class ArtifactString:
    """One interned string carried by an artifact record."""

    def __init__(self, id: str, text: str) -> None: ...

    """Canonical lowercase hex string id."""
    @property
    def id(self) -> str: ...

    """Interned string text."""
    @property
    def text(self) -> str: ...

class ArtifactRecord:
    """Self-contained raw artifact body crossing bridge boundaries."""

    def __init__(self, version: ArtifactVersion, image: bytes | bytearray | Sequence[int], strings: Sequence[ArtifactString], dependencies: Sequence[ArtifactDependency], diagnostics: Sequence[Diagnostic], sidecars: Sequence[ArtifactSidecar]) -> None: ...

    """The exact artifact version."""
    @property
    def version(self) -> ArtifactVersion: ...

    """Serialized artifact image bytes."""
    @property
    def image(self) -> list[int]: ...

    """String pool needed to interpret interned ids in the payload."""
    @property
    def strings(self) -> list[ArtifactString]: ...

    """Exact artifact dependencies."""
    @property
    def dependencies(self) -> list[ArtifactDependency]: ...

    """Diagnostics recorded for this artifact version."""
    @property
    def diagnostics(self) -> list[Diagnostic]: ...

    """Artifact sidecars recorded for this artifact version."""
    @property
    def sidecars(self) -> list[ArtifactSidecar]: ...

