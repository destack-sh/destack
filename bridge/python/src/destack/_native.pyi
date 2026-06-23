# generated bridge target, do not edit

from __future__ import annotations

VERSION: str

def version() -> str: ...

class LocalWorkspaceServer:
    """In-process workspace protocol server."""

    @staticmethod
    def open(home: str) -> LocalWorkspaceServer: ...
    def dispatch(self, payload: bytes) -> list[bytes]: ...
