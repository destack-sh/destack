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

class ArtifactPathState:
    """Exact source path state observed by one artifact computation."""

    @property
    def label(self) -> str: ...

class ArtifactDirectoryEntry:
    """One exact directory entry observed by one artifact computation."""

    """The entry path."""
    @property
    def path(self) -> str: ...

    """The exact entry path state."""
    @property
    def state(self) -> ArtifactPathState: ...

class ArtifactSourceDependency:
    """One primitive source observation read while building an artifact."""

    @property
    def kind(self) -> str: ...

    @property
    def content(self) -> ContentId | None: ...

    @property
    def directory(self) -> str | None: ...

    @property
    def entries(self) -> list[ArtifactDirectoryEntry] | None: ...

    @property
    def file(self) -> FileId | None: ...

    @property
    def path(self) -> str | None: ...

    @property
    def state(self) -> ArtifactPathState | None: ...

class ArtifactDependency:
    """One exact dependency read while building an artifact."""

    @property
    def kind(self) -> str: ...

    @property
    def dependency(self) -> ArtifactSourceDependency | None: ...

    @property
    def version(self) -> ArtifactVersion | None: ...
