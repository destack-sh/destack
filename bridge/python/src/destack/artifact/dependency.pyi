# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence

from destack.artifact.version import (
    ArtifactVersion,
)

from destack.source.file import (
    FileContentId,
    FileId,
)

class ArtifactPathState:
    """Exact source path state observed by one artifact computation."""

    """The path did not exist."""
    @staticmethod
    def missing() -> ArtifactPathState: ...

    """The path was a regular file."""
    @staticmethod
    def file() -> ArtifactPathState: ...

    """The path was a directory."""
    @staticmethod
    def directory() -> ArtifactPathState: ...

    """The path was a symbolic link."""
    @staticmethod
    def symlink() -> ArtifactPathState: ...

    """The path existed with another host-specific kind."""
    @staticmethod
    def other() -> ArtifactPathState: ...

    @property
    def label(self) -> str: ...

class ArtifactDirectoryEntry:
    """One exact directory entry observed by one artifact computation."""

    def __init__(self, path: FileId, state: ArtifactPathState) -> None: ...

    """The entry path identity."""
    @property
    def path(self) -> FileId: ...

    """The exact entry path state."""
    @property
    def state(self) -> ArtifactPathState: ...

class ArtifactSourceDependency:
    """One primitive source observation read while building an artifact."""

    """The exact state observed for one source path."""
    @staticmethod
    def path_state(path: FileId, state: ArtifactPathState) -> ArtifactSourceDependency: ...

    """The exact direct entries observed for one directory."""
    @staticmethod
    def directory_entries(directory: FileId, entries: Sequence[ArtifactDirectoryEntry]) -> ArtifactSourceDependency: ...

    """The exact source content read for one file."""
    @staticmethod
    def file_content(file: FileId, content: FileContentId) -> ArtifactSourceDependency: ...

    @property
    def kind(self) -> str: ...

    @property
    def content(self) -> FileContentId | None: ...

    @property
    def directory(self) -> FileId | None: ...

    @property
    def entries(self) -> list[ArtifactDirectoryEntry] | None: ...

    @property
    def file(self) -> FileId | None: ...

    @property
    def path(self) -> FileId | None: ...

    @property
    def state(self) -> ArtifactPathState | None: ...

class ArtifactDependency:
    """One exact dependency read while building an artifact."""

    """Another exact artifact version."""
    @staticmethod
    def artifact(version: ArtifactVersion) -> ArtifactDependency: ...

    """One exact primitive source observation."""
    @staticmethod
    def source(dependency: ArtifactSourceDependency) -> ArtifactDependency: ...

    @property
    def kind(self) -> str: ...

    @property
    def dependency(self) -> ArtifactSourceDependency | None: ...

    @property
    def version(self) -> ArtifactVersion | None: ...

