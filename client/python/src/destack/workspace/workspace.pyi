# generated client target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import Any, Protocol, TypeAlias, cast

from destack.protocol.connection import Connection
from destack.protocol.serde import Json
from ..protocol.workspace.client import (
    RemoteWorkspace,
    Workspace,
    open_remote_workspace,
)

MemoryContent: TypeAlias = str | bytes | bytearray | Sequence[int]

class JsonValue(Protocol):
    """Value that can render itself as client JSON."""

    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class MemoryFile:
    """One file in an in-memory workspace."""

    path: str
    text: str | None = None
    bytes: bytes | bytearray | Sequence[int] | None = None

@dataclass(frozen=True, slots=True)
class MemoryWorkspace:
    """In-memory workspace source."""

    root: str = "/workspace"
    config: Json | JsonValue | None = None
    files: Mapping[str, MemoryContent] | Sequence[MemoryFile] = ...

def open_workspace(
    *,
    workspace: str | None = None,
    memory: MemoryWorkspace | Mapping[str, Any] | None = None,
    root: str | None = None,
    url: str | None = None,
    connection: Connection | None = None,
    load_index: bool = False,
) -> Workspace: ...
