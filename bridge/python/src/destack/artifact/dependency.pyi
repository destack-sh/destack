# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence

from destack.artifact.version import (
    ArtifactVersion,
)

from destack.source.file import (
    ContentId,
    FileId,
)

class ArtifactDependency:
    """One exact dependency read while building an artifact."""

    @property
    def kind(self) -> str: ...

    @property
    def content(self) -> ContentId | None: ...

    @property
    def file(self) -> FileId | None: ...

    @property
    def version(self) -> ArtifactVersion | None: ...
