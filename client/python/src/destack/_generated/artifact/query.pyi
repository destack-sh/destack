# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.artifact.core.version
import destack._generated.qir.index.index

@dataclass(frozen=True, slots=True)
class ModuleQueryIndex:
    """Query index for one module profile."""

    # the indexed query surface
    index: destack._generated.qir.index.index.QueryIndex

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ModuleQueryIndex: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ModuleQueryIndex: ...

def encode_module_query_index(
    writer: BinaryWriter, value: ModuleQueryIndex
) -> None: ...
def decode_module_query_index(reader: BinaryReader) -> ModuleQueryIndex: ...
def to_json_module_query_index(value: ModuleQueryIndex) -> Json: ...
def from_json_module_query_index(value: Json) -> ModuleQueryIndex: ...

@dataclass(frozen=True, slots=True)
class WorkspaceQueryIndex:
    """Query index for one workspace profile."""

    # the module query indexes in this workspace profile
    modules: Sequence[destack._generated.artifact.core.version.ArtifactVersion]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> WorkspaceQueryIndex: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> WorkspaceQueryIndex: ...

def encode_workspace_query_index(
    writer: BinaryWriter, value: WorkspaceQueryIndex
) -> None: ...
def decode_workspace_query_index(reader: BinaryReader) -> WorkspaceQueryIndex: ...
def to_json_workspace_query_index(value: WorkspaceQueryIndex) -> Json: ...
def from_json_workspace_query_index(value: Json) -> WorkspaceQueryIndex: ...

__all__ = [
    "ModuleQueryIndex",
    "encode_module_query_index",
    "decode_module_query_index",
    "to_json_module_query_index",
    "from_json_module_query_index",
    "WorkspaceQueryIndex",
    "encode_workspace_query_index",
    "decode_workspace_query_index",
    "to_json_workspace_query_index",
    "from_json_workspace_query_index",
]
