# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes

import destack._generated.protocol.query.core.target
import destack._generated.protocol.query.dir.kind
import destack._generated.protocol.source.file.model.span

if TYPE_CHECKING:
    from destack._generated.protocol.query.core.target import (
        QueryModule,
    )

    from destack._generated.protocol.query.dir.kind import (
        SymbolKind,
    )

    from destack._generated.protocol.source.file.model.span import (
        Span,
    )


@dataclass(frozen=True, slots=True)
class DocumentSymbolsRequest:
    """Request document symbols for a document."""

    """The queried module."""
    module: QueryModule


def encode_document_symbols_request(
    writer: Writer, value: DocumentSymbolsRequest
) -> None:
    destack._generated.protocol.query.core.target.encode_query_module(
        writer, value.module
    )


def decode_document_symbols_request(reader: Reader) -> DocumentSymbolsRequest:
    field_0 = destack._generated.protocol.query.core.target.decode_query_module(reader)

    return DocumentSymbolsRequest(
        module=field_0,
    )


@dataclass(frozen=True, slots=True)
class DocumentSymbolsResponse:
    """Response payload for document symbols queries."""

    """Document symbols."""
    symbols: Sequence[DocumentSymbol]


def encode_document_symbols_response(
    writer: Writer, value: DocumentSymbolsResponse
) -> None:
    writer.write_unsigned(len(value.symbols))
    for item_0 in value.symbols:
        encode_document_symbol(writer, item_0)


def decode_document_symbols_response(reader: Reader) -> DocumentSymbolsResponse:
    field_0 = [decode_document_symbol(reader) for _ in range(reader.read_number())]

    return DocumentSymbolsResponse(
        symbols=field_0,
    )


@dataclass(frozen=True, slots=True)
class DocumentSymbol:
    """A symbol in a document (for outline view)."""

    """The symbol's name."""
    name: str
    """Additional detail (e.g., signature)."""
    detail: str | None
    """The kind of symbol."""
    kind: SymbolKind
    """The full range of the symbol (including body)."""
    range: Span
    """The range of the symbol's name."""
    selection_range: Span
    """Children symbols (for hierarchical outline)."""
    children: Sequence[DocumentSymbol]


def encode_document_symbol(writer: Writer, value: DocumentSymbol) -> None:
    writer.write_string(value.name)
    if value.detail is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.detail)
    destack._generated.protocol.query.dir.kind.encode_symbol_kind(writer, value.kind)
    destack._generated.protocol.source.file.model.span.encode_span(writer, value.range)
    destack._generated.protocol.source.file.model.span.encode_span(
        writer, value.selection_range
    )
    writer.write_unsigned(len(value.children))
    for item_0 in value.children:
        encode_document_symbol(writer, item_0)


def decode_document_symbol(reader: Reader) -> DocumentSymbol:
    field_0 = reader.read_string()
    field_1 = reader.read_option(lambda: reader.read_string())
    field_2 = destack._generated.protocol.query.dir.kind.decode_symbol_kind(reader)
    field_3 = destack._generated.protocol.source.file.model.span.decode_span(reader)
    field_4 = destack._generated.protocol.source.file.model.span.decode_span(reader)
    field_5 = [decode_document_symbol(reader) for _ in range(reader.read_number())]

    return DocumentSymbol(
        name=field_0,
        detail=field_1,
        kind=field_2,
        range=field_3,
        selection_range=field_4,
        children=field_5,
    )


__all__ = [
    "DocumentSymbolsRequest",
    "encode_document_symbols_request",
    "decode_document_symbols_request",
    "DocumentSymbolsResponse",
    "encode_document_symbols_response",
    "decode_document_symbols_response",
    "DocumentSymbol",
    "encode_document_symbol",
    "decode_document_symbol",
]
