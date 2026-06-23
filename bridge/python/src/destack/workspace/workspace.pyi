# generated bridge target, do not edit

from __future__ import annotations

from destack.protocol.connection import Connection
from ..protocol.workspace.client import (
    RemoteWorkspace,
    Workspace,
    open_remote_workspace,
)

def open_workspace(
    *,
    workspace: str,
    root: str | None = None,
    url: str | None = None,
    connection: Connection | None = None,
    load_index: bool = False,
) -> Workspace: ...
