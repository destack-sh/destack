import abc
from typing import Mapping, Optional, cast
from uuid import UUID

from fsspec import AbstractFileSystem

from bench.utils.spec import ArtifactType, FieldSpec

NO_STATIC_KEYS = cast(set[str], set())


class ArtifactHandler(abc.ABC):
    """
    Base for interacting with artifacts.

    Artifacts are configured
    """

    # Config spec to configure this handler.
    config_spec: Mapping[str, FieldSpec]
    # Config values are immutable after init. If not set, defaults to all keys.
    config_static_keys: set[str]
    # Three types of spec:
    #  - base_spec applies to all artifacts connected to this handler
    #  - spec may be configured and applies to this specific artifact and handler
    #  - runtime_spec is derived from this specific artifact at runtime
    # Generic base spec for this handler (before configuration).
    base_spec: Optional[ArtifactType]
    # Generic spec for this handler.
    spec: ArtifactType

    def __init__(
        self,
        artifact_id: UUID,
        version: Optional[str],
        fs: Optional[AbstractFileSystem],
        path: Optional[str],
    ):
        """
        Creates a handler for this artifact using the given storage.

        @param fs: The file system to use (if any).
        @param path: The configured path within the file system (if any).
        @param version: The version of this artifact (if any).
        """

        self._fs = fs
        self._path = path
        self._version = version

    @property
    def runtime_spec(self) -> ArtifactType:
        """Gets the actual *runtime* spec of the current model (if different from configured)"""
        return self.base_spec or self.spec

    @property
    def fs(self) -> AbstractFileSystem:
        if self._fs is None:
            raise ValueError("file system is not available")
        return self._fs

    @property
    def path(self) -> str:
        if self._path is None:
            raise ValueError("path is not set")
        return self._path
