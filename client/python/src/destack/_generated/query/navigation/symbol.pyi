# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.dir.symbol.symbol
import destack._generated.query.protocol.target
import destack._generated.source.file.model.span

@dataclass(frozen=True, slots=True)
class OutlineRequest:
    """Request symbols for a module."""

    # the queried module
    module: destack._generated.query.protocol.target.Module

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> OutlineRequest: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> OutlineRequest: ...

def encode_outline_request(writer: BinaryWriter, value: OutlineRequest) -> None: ...
def decode_outline_request(reader: BinaryReader) -> OutlineRequest: ...
def to_json_outline_request(value: OutlineRequest) -> Json: ...
def from_json_outline_request(value: Json) -> OutlineRequest: ...

@dataclass(frozen=True, slots=True)
class OutlineResponse:
    """Response payload for symbols queries."""

    # symbols
    symbols: Sequence[Symbol]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> OutlineResponse: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> OutlineResponse: ...

def encode_outline_response(writer: BinaryWriter, value: OutlineResponse) -> None: ...
def decode_outline_response(reader: BinaryReader) -> OutlineResponse: ...
def to_json_outline_response(value: OutlineResponse) -> Json: ...
def from_json_outline_response(value: Json) -> OutlineResponse: ...

@dataclass(frozen=True, slots=True)
class Symbol:
    """A symbol in a module (for outline view)."""

    # the symbol's name
    name: str
    # additional detail (e.g., signature)
    detail: str | None
    # the kind of symbol
    kind: destack._generated.dir.symbol.symbol.SymbolKind
    # the full range of the symbol (including body)
    range: destack._generated.source.file.model.span.Span
    # the range of the symbol's name
    selection_range: destack._generated.source.file.model.span.Span
    # children symbols (for hierarchical outline)
    children: Sequence[Symbol]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Symbol: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Symbol: ...

def encode_symbol(writer: BinaryWriter, value: Symbol) -> None: ...
def decode_symbol(reader: BinaryReader) -> Symbol: ...
def to_json_symbol(value: Symbol) -> Json: ...
def from_json_symbol(value: Json) -> Symbol: ...

__all__ = [
    "OutlineRequest",
    "encode_outline_request",
    "decode_outline_request",
    "to_json_outline_request",
    "from_json_outline_request",
    "OutlineResponse",
    "encode_outline_response",
    "decode_outline_response",
    "to_json_outline_response",
    "from_json_outline_response",
    "Symbol",
    "encode_symbol",
    "decode_symbol",
    "to_json_symbol",
    "from_json_symbol",
]
