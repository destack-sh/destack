# generated bridge target, do not edit

from __future__ import annotations

VERSION: str

def version() -> str: ...

class LocalWorkspaceServer:
    """In-process workspace protocol server."""

    @staticmethod
    def open(home: str) -> LocalWorkspaceServer: ...
    @staticmethod
    def memory(
        root: str, text_files: dict[str, str], byte_files: dict[str, bytes]
    ) -> LocalWorkspaceServer: ...
    def dispatch(self, payload: bytes) -> list[bytes]: ...

class RemoteWorkspaceServer:
    """Remote workspace protocol server."""

    @staticmethod
    def open(root: str) -> RemoteWorkspaceServer: ...
    def url(self) -> str: ...
    def close(self) -> None: ...
