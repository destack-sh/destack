from __future__ import annotations

import json
from collections.abc import Iterator, Mapping
from contextlib import AbstractContextManager, contextmanager
from pathlib import Path
from tempfile import TemporaryDirectory
from typing import Any, Protocol

from destack._native import RemoteWorkspaceServer
from destack.workspace import MemoryWorkspace, Workspace, open_workspace


def workspace_modes() -> list[tuple[str, WorkspaceOpener]]:
    """Return client workspace test modes."""

    return [
        ("local", local_workspace),
        ("remote", remote_workspace),
    ]


class WorkspaceOpener(Protocol):
    """Callable that opens one client workspace test mode."""

    def __call__(
        self, *, config: Mapping[str, Any], files: Mapping[str, str]
    ) -> AbstractContextManager[Workspace]:
        """Open one client workspace test mode."""


@contextmanager
def local_workspace(
    *, config: Mapping[str, Any], files: Mapping[str, str]
) -> Iterator[Workspace]:
    """Open one embedded local workspace."""

    workspace = open_workspace(
        memory=MemoryWorkspace(
            config=config,
            files=files,
        )
    )

    try:
        yield workspace

    finally:
        workspace.close()


@contextmanager
def remote_workspace(
    *, config: Mapping[str, Any], files: Mapping[str, str]
) -> Iterator[Workspace]:
    """Open one remote workspace through an in-process protocol server."""

    with TemporaryDirectory(prefix="destack-client-project-") as root:
        root_path = Path(root)
        write_project(root_path, config, files)
        server = RemoteWorkspaceServer.open(str(root_path))

        try:
            workspace = open_workspace(
                url=server.url(),
                workspace=str(root_path),
            )

            try:
                yield workspace

            finally:
                try:
                    workspace.close()

                finally:
                    workspace.connection().close()

        finally:
            server.close()


def write_project(
    root: Path, config: Mapping[str, Any], files: Mapping[str, str]
) -> None:
    """Write one project tree to a temporary directory."""

    config_text = json.dumps(config, separators=(",", ":"))
    (root / "destack.json").write_text(f"{config_text}\n", encoding="utf-8")

    for path, text in files.items():
        file_path = root / path
        file_path.parent.mkdir(parents=True, exist_ok=True)
        file_path.write_text(text, encoding="utf-8")
