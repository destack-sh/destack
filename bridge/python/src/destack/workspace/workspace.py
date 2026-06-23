# generated bridge target, do not edit

from __future__ import annotations

from destack._native import LocalWorkspaceServer
from destack.protocol.connection import EmbeddedTransport, Connection
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
) -> Workspace:
    """Open one workspace through a local or remote transport."""

    if url is not None or connection is not None:
        return open_remote_workspace(
            workspace=workspace,
            root=root,
            url=url,
            connection=connection,
            load_index=load_index,
        )

    server = LocalWorkspaceServer.open(workspace)
    transport = EmbeddedTransport(server)
    connection = Connection(transport)

    return open_remote_workspace(
        workspace=workspace,
        root=root,
        connection=connection,
        load_index=load_index,
    )


__all__ = [
    "Workspace",
    "RemoteWorkspace",
    "open_workspace",
]
