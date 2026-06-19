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

    """Canonical lowercase hex string id."""
    @property
    def id(self) -> str: ...

    """Interned string text."""
    @property
    def text(self) -> str: ...

class ArtifactRecord:
    """Self-contained raw artifact body crossing bridge boundaries."""

    """The exact artifact version."""
    @property
    def version(self) -> ArtifactVersion: ...

    """The predecessor artifact this record was incrementally built from."""
    @property
    def base(self) -> ArtifactVersion | None: ...

    """Serialized artifact payload bytes."""
    @property
    def payload(self) -> list[int]: ...

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
