# generated bridge target, do not edit

from __future__ import annotations

import json
from collections.abc import Mapping, Sequence
from dataclasses import dataclass, field
from typing import Any, Protocol, TypeAlias, cast

from destack._native import LocalWorkspaceServer
from destack.protocol.connection import EmbeddedTransport, Connection
from destack.protocol.serde import Json
from ..protocol.workspace.client import (
    RemoteWorkspace,
    Workspace,
    open_remote_workspace,
)

MemoryContent: TypeAlias = str | bytes | bytearray | Sequence[int]


class JsonValue(Protocol):
    """Value that can render itself as bridge JSON."""

    def to_json(self) -> Json:
        """Return this value as JSON."""


@dataclass(frozen=True, slots=True)
class MemoryFile:
    """One file in an in-memory workspace."""

    # repository relative file path
    path: str
    # UTF-8 text content
    text: str | None = None
    # binary content
    bytes: bytes | bytearray | Sequence[int] | None = None


@dataclass(frozen=True, slots=True)
class MemoryWorkspace:
    """In-memory workspace source."""

    # in-memory workspace root path
    root: str = "/workspace"
    # typed destack.json content
    config: Json | JsonValue | None = None
    # in-memory repository files
    files: Mapping[str, MemoryContent] | Sequence[MemoryFile] = field(
        default_factory=dict
    )


def memory_workspace(memory: MemoryWorkspace | Mapping[str, Any]) -> MemoryWorkspace:
    """Return one normalized memory workspace."""

    if isinstance(memory, MemoryWorkspace):
        return memory

    return MemoryWorkspace(
        root=memory.get("root", "/workspace"),
        config=memory.get("config"),
        files=memory.get("files", {}),
    )


def memory_files(memory: MemoryWorkspace) -> tuple[dict[str, str], dict[str, bytes]]:
    """Return memory workspace files split by content kind."""

    text_files: dict[str, str] = {}
    byte_files: dict[str, bytes] = {}

    if memory.config is not None:
        config = json.dumps(
            config_json(memory.config),
            separators=(",", ":"),
            ensure_ascii=False,
        )
        text_files["destack.json"] = f"{config}\n"

    for file in explicit_memory_files(memory.files):
        if file.path == "destack.json" and memory.config is not None:
            raise ValueError(
                "memory workspace cannot define both config and destack.json"
            )

        if file.text is not None and file.bytes is None:
            text_files[file.path] = file.text

        elif file.text is None and file.bytes is not None:
            byte_files[file.path] = bytes(file.bytes)

        else:
            raise ValueError("memory file must contain exactly one content value")

    return text_files, byte_files


def explicit_memory_files(
    files: Mapping[str, MemoryContent] | Sequence[MemoryFile],
) -> Sequence[MemoryFile]:
    """Return explicit memory files from either mapping or sequence form."""

    if isinstance(files, Mapping):
        files = cast(Mapping[str, MemoryContent], files)
        return [memory_file(path, content) for path, content in files.items()]

    return files


def memory_file(path: str, content: MemoryContent) -> MemoryFile:
    """Return one memory file from short mapping content."""

    if isinstance(content, str):
        return MemoryFile(path=path, text=content)

    return MemoryFile(path=path, bytes=content)


def config_json(config: Json | JsonValue) -> Json:
    """Return one config value as JSON."""

    if hasattr(config, "to_json"):
        return config.to_json()

    return config


def open_workspace(
    *,
    workspace: str | None = None,
    memory: MemoryWorkspace | Mapping[str, Any] | None = None,
    root: str | None = None,
    url: str | None = None,
    connection: Connection | None = None,
    load_index: bool = False,
) -> Workspace:
    """Open one workspace through a local or remote transport."""

    if memory is not None and (workspace is not None or root is not None):
        raise ValueError("memory workspace cannot also specify workspace or root")

    if url is not None or connection is not None:
        if workspace is None:
            raise ValueError("remote workspace requires workspace")

        return open_remote_workspace(
            workspace=workspace,
            root=root,
            url=url,
            connection=connection,
            load_index=load_index,
        )

    if memory is None:
        if workspace is None:
            raise ValueError("local workspace requires workspace or memory")

        server = LocalWorkspaceServer.open(workspace)

    else:
        memory = memory_workspace(memory)
        text_files, byte_files = memory_files(memory)
        server = LocalWorkspaceServer.memory(memory.root, text_files, byte_files)
        workspace = memory.root

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
    "MemoryContent",
    "MemoryFile",
    "MemoryWorkspace",
    "open_workspace",
]
