# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_array,
    json_field,
    json_object,
    json_optional,
    json_string,
)

import destack._generated.query.core.target
import destack._generated.query.dir.kind
import destack._generated.source.file.model.span


@dataclass(frozen=True, slots=True)
class DocumentSymbolsRequest:
    """Request document symbols for a document."""

    # the queried module
    module: destack._generated.query.core.target.QueryModule

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_document_symbols_request(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> DocumentSymbolsRequest:
        """Decode one DocumentSymbolsRequest."""
        return decode_document_symbols_request(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_document_symbols_request(self)

    @classmethod
    def from_json(cls, value: Json) -> DocumentSymbolsRequest:
        """Return one DocumentSymbolsRequest from one JSON value."""
        return from_json_document_symbols_request(value)


def encode_document_symbols_request(
    writer: BinaryWriter, value: DocumentSymbolsRequest
) -> None:
    """Encode one DocumentSymbolsRequest."""
    destack._generated.query.core.target.encode_query_module(writer, value.module)


def decode_document_symbols_request(reader: BinaryReader) -> DocumentSymbolsRequest:
    """Decode one DocumentSymbolsRequest."""
    module = destack._generated.query.core.target.decode_query_module(reader)

    return DocumentSymbolsRequest(
        module=module,
    )


def to_json_document_symbols_request(value: DocumentSymbolsRequest) -> Json:
    """Return one JSON value for one DocumentSymbolsRequest."""
    return {
        "module": destack._generated.query.core.target.to_json_query_module(
            value.module
        ),
    }


def from_json_document_symbols_request(value: Json) -> DocumentSymbolsRequest:
    """Return one DocumentSymbolsRequest from one JSON value."""
    object_ = json_object(value)

    return DocumentSymbolsRequest(
        module=destack._generated.query.core.target.from_json_query_module(
            json_field(object_, "module")
        ),
    )


@dataclass(frozen=True, slots=True)
class DocumentSymbolsResponse:
    """Response payload for document symbols queries."""

    # document symbols
    symbols: Sequence[DocumentSymbol]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_document_symbols_response(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> DocumentSymbolsResponse:
        """Decode one DocumentSymbolsResponse."""
        return decode_document_symbols_response(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_document_symbols_response(self)

    @classmethod
    def from_json(cls, value: Json) -> DocumentSymbolsResponse:
        """Return one DocumentSymbolsResponse from one JSON value."""
        return from_json_document_symbols_response(value)


def encode_document_symbols_response(
    writer: BinaryWriter, value: DocumentSymbolsResponse
) -> None:
    """Encode one DocumentSymbolsResponse."""
    writer.write_unsigned(len(value.symbols))
    for item_value_symbols_0 in value.symbols:
        encode_document_symbol(writer, item_value_symbols_0)


def decode_document_symbols_response(reader: BinaryReader) -> DocumentSymbolsResponse:
    """Decode one DocumentSymbolsResponse."""
    symbols = [decode_document_symbol(reader) for _ in range(reader.read_number())]

    return DocumentSymbolsResponse(
        symbols=symbols,
    )


def to_json_document_symbols_response(value: DocumentSymbolsResponse) -> Json:
    """Return one JSON value for one DocumentSymbolsResponse."""
    return {
        "symbols": [to_json_document_symbol(item_0) for item_0 in value.symbols],
    }


def from_json_document_symbols_response(value: Json) -> DocumentSymbolsResponse:
    """Return one DocumentSymbolsResponse from one JSON value."""
    object_ = json_object(value)

    return DocumentSymbolsResponse(
        symbols=[
            from_json_document_symbol(item_0)
            for item_0 in json_array(json_field(object_, "symbols"))
        ],
    )


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_document_symbol(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> DocumentSymbol:
        """Decode one DocumentSymbol."""
        return decode_document_symbol(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_document_symbol(self)

    @classmethod
    def from_json(cls, value: Json) -> DocumentSymbol:
        """Return one DocumentSymbol from one JSON value."""
        return from_json_document_symbol(value)


def encode_document_symbol(writer: BinaryWriter, value: DocumentSymbol) -> None:
    """Encode one DocumentSymbol."""
    writer.write_string(value.name)
    if value.detail is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.detail)
    destack._generated.query.dir.kind.encode_symbol_kind(writer, value.kind)
    destack._generated.source.file.model.span.encode_span(writer, value.range)
    destack._generated.source.file.model.span.encode_span(writer, value.selection_range)
    writer.write_unsigned(len(value.children))
    for item_value_children_0 in value.children:
        encode_document_symbol(writer, item_value_children_0)


def decode_document_symbol(reader: BinaryReader) -> DocumentSymbol:
    """Decode one DocumentSymbol."""
    name = reader.read_string()
    detail = reader.read_option(lambda: reader.read_string())
    kind = destack._generated.query.dir.kind.decode_symbol_kind(reader)
    range_ = destack._generated.source.file.model.span.decode_span(reader)
    selection_range = destack._generated.source.file.model.span.decode_span(reader)
    children = [decode_document_symbol(reader) for _ in range(reader.read_number())]

    return DocumentSymbol(
        name=name,
        detail=detail,
        kind=kind,
        range=range_,
        selection_range=selection_range,
        children=children,
    )


def to_json_document_symbol(value: DocumentSymbol) -> Json:
    """Return one JSON value for one DocumentSymbol."""
    return {
        "name": value.name,
        **({} if value.detail is None else {"detail": value.detail}),
        "kind": destack._generated.query.dir.kind.to_json_symbol_kind(value.kind),
        "range": destack._generated.source.file.model.span.to_json_span(value.range),
        "selectionRange": destack._generated.source.file.model.span.to_json_span(
            value.selection_range
        ),
        "children": [to_json_document_symbol(item_0) for item_0 in value.children],
    }


def from_json_document_symbol(value: Json) -> DocumentSymbol:
    """Return one DocumentSymbol from one JSON value."""
    object_ = json_object(value)

    return DocumentSymbol(
        name=json_string(json_field(object_, "name")),
        detail=json_optional(object_, "detail", lambda value: json_string(value)),
        kind=destack._generated.query.dir.kind.from_json_symbol_kind(
            json_field(object_, "kind")
        ),
        range=destack._generated.source.file.model.span.from_json_span(
            json_field(object_, "range")
        ),
        selection_range=destack._generated.source.file.model.span.from_json_span(
            json_field(object_, "selectionRange")
        ),
        children=[
            from_json_document_symbol(item_0)
            for item_0 in json_array(json_field(object_, "children"))
        ],
    )


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
