import abc
from typing import Optional, cast

from fsspec import AbstractFileSystem

from bench.utils.spec import ArtifactSpec, ConfigSpec

NO_STATIC_KEYS = cast(set[str], set())


class ArtifactHandler(abc.ABC):
    """
    Base for interacting with artifacts.

    Artifacts are configured
    """

    # Config spec to configure this handler.
    config_spec: ConfigSpec
    # Config values are immutable after init. If not set, defaults to all keys.
    config_static_keys: set[str]
    # Generic spec for this handler.
    spec: ArtifactSpec

    def __init__(
        self,
        fs: Optional[AbstractFileSystem],
        path: Optional[str],
        version: Optional[str],
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
    def runtime_spec(self) -> ArtifactSpec:
        """Gets the actual *runtime* spec of the current model (if different from configured)"""
        return self.spec

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


class ArtifactVersionHandler(ArtifactHandler):
    """
    Base for interacting with versioned artifacts.
    """

    @property
    def version(self) -> str:
        if self._version is None:
            raise ValueError(f"artifact is not versioned: {self}")
        return self._version

    @property
    def history(self) -> list[str]:
        """
        Gets the linear log of versions
        TODO @Feature: support branches in artifact version handler
        """
        raise NotImplementedError

    def commit(self):
        """
        Commits the current version as immutable.
        """
        raise NotImplementedError

    def checkout(self, version: str) -> "ArtifactVersionHandler":
        """
        Returns a new handler with the given version checked out (creating it if necessary), where
        the new version is based on the state of this version.
        TODO @Feature: ArtifactVersionHandler.checkout should have create flag?
         (as in, whether to create a new version from the current or check out an existing version)
        """
        raise NotImplementedError
