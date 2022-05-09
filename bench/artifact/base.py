import abc
from typing import Optional

from fsspec import AbstractFileSystem

from bench.utils.spec import ConfigSpec


class ArtifactHandler(abc.ABC):
    """
    Base for interacting with artifacts.
    """

    # Config spec to configure this handler.
    config_spec: ConfigSpec
    # Config values are immutable after init. If not set, defaults to all keys.
    config_static_keys: set[str]

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
    def fs(self) -> AbstractFileSystem:
        if self._fs is None:
            raise ValueError("file system is not available")
        return self._fs

    @property
    def path(self) -> str:
        if self._path is None:
            raise ValueError("path is not set")
        return self._path
