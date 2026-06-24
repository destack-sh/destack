# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.query.core.target
import destack._generated.query.dir.kind
import destack._generated.source.file.model.profile

@dataclass(frozen=True, slots=True)
class WorkspaceSymbolsRequest:
    """Request workspace symbols for a query string."""

    # the profiles to search
    profile_ids: Sequence[destack._generated.source.file.model.profile.ProfileId]
    # the search query string
    query: str
    # the maximum number of results
    max_results: int

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> WorkspaceSymbolsRequest: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> WorkspaceSymbolsRequest: ...

def encode_workspace_symbols_request(
    writer: BinaryWriter, value: WorkspaceSymbolsRequest
) -> None: ...
def decode_workspace_symbols_request(
    reader: BinaryReader,
) -> WorkspaceSymbolsRequest: ...
def to_json_workspace_symbols_request(value: WorkspaceSymbolsRequest) -> Json: ...
def from_json_workspace_symbols_request(value: Json) -> WorkspaceSymbolsRequest: ...

@dataclass(frozen=True, slots=True)
class WorkspaceSymbolsResponse:
    """Response payload for workspace symbols queries."""

    # workspace symbols
    symbols: Sequence[WorkspaceSymbol]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> WorkspaceSymbolsResponse: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> WorkspaceSymbolsResponse: ...

def encode_workspace_symbols_response(
    writer: BinaryWriter, value: WorkspaceSymbolsResponse
) -> None: ...
def decode_workspace_symbols_response(
    reader: BinaryReader,
) -> WorkspaceSymbolsResponse: ...
def to_json_workspace_symbols_response(value: WorkspaceSymbolsResponse) -> Json: ...
def from_json_workspace_symbols_response(value: Json) -> WorkspaceSymbolsResponse: ...

@dataclass(frozen=True, slots=True)
class WorkspaceSymbol:
    """A symbol in the workspace (flat list for workspace symbol search)."""

    # the symbol's name
    name: str
    # the kind of symbol
    kind: destack._generated.query.dir.kind.SymbolKind
    # the symbol source target
    target: destack._generated.query.core.target.QueryTarget
    # container name (e.g., class name for methods)
    container: str | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> WorkspaceSymbol: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> WorkspaceSymbol: ...

def encode_workspace_symbol(writer: BinaryWriter, value: WorkspaceSymbol) -> None: ...
def decode_workspace_symbol(reader: BinaryReader) -> WorkspaceSymbol: ...
def to_json_workspace_symbol(value: WorkspaceSymbol) -> Json: ...
def from_json_workspace_symbol(value: Json) -> WorkspaceSymbol: ...

__all__ = [
    "WorkspaceSymbolsRequest",
    "encode_workspace_symbols_request",
    "decode_workspace_symbols_request",
    "to_json_workspace_symbols_request",
    "from_json_workspace_symbols_request",
    "WorkspaceSymbolsResponse",
    "encode_workspace_symbols_response",
    "decode_workspace_symbols_response",
    "to_json_workspace_symbols_response",
    "from_json_workspace_symbols_response",
    "WorkspaceSymbol",
    "encode_workspace_symbol",
    "decode_workspace_symbol",
    "to_json_workspace_symbol",
    "from_json_workspace_symbol",
]
