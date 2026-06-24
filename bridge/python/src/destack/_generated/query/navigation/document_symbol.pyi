# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.query.core.target
import destack._generated.query.dir.kind
import destack._generated.source.file.model.span

@dataclass(frozen=True, slots=True)
class DocumentSymbolsRequest:
    """Request document symbols for a document."""

    # the queried module
    module: destack._generated.query.core.target.QueryModule

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> DocumentSymbolsRequest: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> DocumentSymbolsRequest: ...

def encode_document_symbols_request(
    writer: BinaryWriter, value: DocumentSymbolsRequest
) -> None: ...
def decode_document_symbols_request(reader: BinaryReader) -> DocumentSymbolsRequest: ...
def to_json_document_symbols_request(value: DocumentSymbolsRequest) -> Json: ...
def from_json_document_symbols_request(value: Json) -> DocumentSymbolsRequest: ...

@dataclass(frozen=True, slots=True)
class DocumentSymbolsResponse:
    """Response payload for document symbols queries."""

    # document symbols
    symbols: Sequence[DocumentSymbol]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> DocumentSymbolsResponse: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> DocumentSymbolsResponse: ...

def encode_document_symbols_response(
    writer: BinaryWriter, value: DocumentSymbolsResponse
) -> None: ...
def decode_document_symbols_response(
    reader: BinaryReader,
) -> DocumentSymbolsResponse: ...
def to_json_document_symbols_response(value: DocumentSymbolsResponse) -> Json: ...
def from_json_document_symbols_response(value: Json) -> DocumentSymbolsResponse: ...

@dataclass(frozen=True, slots=True)
class DocumentSymbol:
    """A symbol in a document (for outline view)."""

    # the symbol's name
    name: str
    # additional detail (e.g., signature)
    detail: str | None
    # the kind of symbol
    kind: destack._generated.query.dir.kind.SymbolKind
    # the full range of the symbol (including body)
    range: destack._generated.source.file.model.span.Span
    # the range of the symbol's name
    selection_range: destack._generated.source.file.model.span.Span
    # children symbols (for hierarchical outline)
    children: Sequence[DocumentSymbol]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> DocumentSymbol: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> DocumentSymbol: ...

def encode_document_symbol(writer: BinaryWriter, value: DocumentSymbol) -> None: ...
def decode_document_symbol(reader: BinaryReader) -> DocumentSymbol: ...
def to_json_document_symbol(value: DocumentSymbol) -> Json: ...
def from_json_document_symbol(value: Json) -> DocumentSymbol: ...

__all__ = [
    "DocumentSymbolsRequest",
    "encode_document_symbols_request",
    "decode_document_symbols_request",
    "to_json_document_symbols_request",
    "from_json_document_symbols_request",
    "DocumentSymbolsResponse",
    "encode_document_symbols_response",
    "decode_document_symbols_response",
    "to_json_document_symbols_response",
    "from_json_document_symbols_response",
    "DocumentSymbol",
    "encode_document_symbol",
    "decode_document_symbol",
    "to_json_document_symbol",
    "from_json_document_symbol",
]
