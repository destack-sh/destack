# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.dir.symbol.symbol
import destack._generated.query.protocol.target
import destack._generated.source.file.model.profile

@dataclass(frozen=True, slots=True)
class SymbolSearchRequest:
    """Request symbol search for a query string."""

    # the profiles to search
    profile_ids: Sequence[destack._generated.source.file.model.profile.ProfileId]
    # the search query string
    query: str
    # the maximum number of results
    max_results: int

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> SymbolSearchRequest: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> SymbolSearchRequest: ...

def encode_symbol_search_request(
    writer: BinaryWriter, value: SymbolSearchRequest
) -> None: ...
def decode_symbol_search_request(reader: BinaryReader) -> SymbolSearchRequest: ...
def to_json_symbol_search_request(value: SymbolSearchRequest) -> Json: ...
def from_json_symbol_search_request(value: Json) -> SymbolSearchRequest: ...

@dataclass(frozen=True, slots=True)
class SymbolSearchResponse:
    """Response payload for symbol search queries."""

    # the matching symbols
    symbols: Sequence[SymbolMatch]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> SymbolSearchResponse: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> SymbolSearchResponse: ...

def encode_symbol_search_response(
    writer: BinaryWriter, value: SymbolSearchResponse
) -> None: ...
def decode_symbol_search_response(reader: BinaryReader) -> SymbolSearchResponse: ...
def to_json_symbol_search_response(value: SymbolSearchResponse) -> Json: ...
def from_json_symbol_search_response(value: Json) -> SymbolSearchResponse: ...

@dataclass(frozen=True, slots=True)
class SymbolMatch:
    """One symbol search result."""

    # the symbol's name
    name: str
    # the kind of symbol
    kind: destack._generated.dir.symbol.symbol.SymbolKind
    # the symbol source target
    target: destack._generated.query.protocol.target.Target
    # container name (e.g., class name for methods)
    container: str | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> SymbolMatch: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> SymbolMatch: ...

def encode_symbol_match(writer: BinaryWriter, value: SymbolMatch) -> None: ...
def decode_symbol_match(reader: BinaryReader) -> SymbolMatch: ...
def to_json_symbol_match(value: SymbolMatch) -> Json: ...
def from_json_symbol_match(value: Json) -> SymbolMatch: ...

__all__ = [
    "SymbolSearchRequest",
    "encode_symbol_search_request",
    "decode_symbol_search_request",
    "to_json_symbol_search_request",
    "from_json_symbol_search_request",
    "SymbolSearchResponse",
    "encode_symbol_search_response",
    "decode_symbol_search_response",
    "to_json_symbol_search_response",
    "from_json_symbol_search_response",
    "SymbolMatch",
    "encode_symbol_match",
    "decode_symbol_match",
    "to_json_symbol_match",
    "from_json_symbol_match",
]
