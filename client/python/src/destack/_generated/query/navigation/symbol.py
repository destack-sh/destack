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

import destack._generated.dir.symbol.symbol
import destack._generated.query.protocol.target
import destack._generated.source.file.model.span


@dataclass(frozen=True, slots=True)
class OutlineRequest:
    """Request symbols for a module."""

    # the queried module
    module: destack._generated.query.protocol.target.Module

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_outline_request(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> OutlineRequest:
        """Decode one OutlineRequest."""
        return decode_outline_request(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_outline_request(self)

    @classmethod
    def from_json(cls, value: Json) -> OutlineRequest:
        """Return one OutlineRequest from one JSON value."""
        return from_json_outline_request(value)


def encode_outline_request(writer: BinaryWriter, value: OutlineRequest) -> None:
    """Encode one OutlineRequest."""
    destack._generated.query.protocol.target.encode_module(writer, value.module)


def decode_outline_request(reader: BinaryReader) -> OutlineRequest:
    """Decode one OutlineRequest."""
    module = destack._generated.query.protocol.target.decode_module(reader)

    return OutlineRequest(
        module=module,
    )


def to_json_outline_request(value: OutlineRequest) -> Json:
    """Return one JSON value for one OutlineRequest."""
    return {
        "module": destack._generated.query.protocol.target.to_json_module(value.module),
    }


def from_json_outline_request(value: Json) -> OutlineRequest:
    """Return one OutlineRequest from one JSON value."""
    object_ = json_object(value)

    return OutlineRequest(
        module=destack._generated.query.protocol.target.from_json_module(
            json_field(object_, "module")
        ),
    )


@dataclass(frozen=True, slots=True)
class OutlineResponse:
    """Response payload for symbols queries."""

    # symbols
    symbols: Sequence[Symbol]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_outline_response(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> OutlineResponse:
        """Decode one OutlineResponse."""
        return decode_outline_response(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_outline_response(self)

    @classmethod
    def from_json(cls, value: Json) -> OutlineResponse:
        """Return one OutlineResponse from one JSON value."""
        return from_json_outline_response(value)


def encode_outline_response(writer: BinaryWriter, value: OutlineResponse) -> None:
    """Encode one OutlineResponse."""
    writer.write_unsigned(len(value.symbols))
    for item_value_symbols_0 in value.symbols:
        encode_symbol(writer, item_value_symbols_0)


def decode_outline_response(reader: BinaryReader) -> OutlineResponse:
    """Decode one OutlineResponse."""
    symbols = [decode_symbol(reader) for _ in range(reader.read_number())]

    return OutlineResponse(
        symbols=symbols,
    )


def to_json_outline_response(value: OutlineResponse) -> Json:
    """Return one JSON value for one OutlineResponse."""
    return {
        "symbols": [to_json_symbol(item_0) for item_0 in value.symbols],
    }


def from_json_outline_response(value: Json) -> OutlineResponse:
    """Return one OutlineResponse from one JSON value."""
    object_ = json_object(value)

    return OutlineResponse(
        symbols=[
            from_json_symbol(item_0)
            for item_0 in json_array(json_field(object_, "symbols"))
        ],
    )


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_symbol(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Symbol:
        """Decode one Symbol."""
        return decode_symbol(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_symbol(self)

    @classmethod
    def from_json(cls, value: Json) -> Symbol:
        """Return one Symbol from one JSON value."""
        return from_json_symbol(value)


def encode_symbol(writer: BinaryWriter, value: Symbol) -> None:
    """Encode one Symbol."""
    writer.write_string(value.name)
    if value.detail is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.detail)
    destack._generated.dir.symbol.symbol.encode_symbol_kind(writer, value.kind)
    destack._generated.source.file.model.span.encode_span(writer, value.range)
    destack._generated.source.file.model.span.encode_span(writer, value.selection_range)
    writer.write_unsigned(len(value.children))
    for item_value_children_0 in value.children:
        encode_symbol(writer, item_value_children_0)


def decode_symbol(reader: BinaryReader) -> Symbol:
    """Decode one Symbol."""
    name = reader.read_string()
    detail = reader.read_option(lambda: reader.read_string())
    kind = destack._generated.dir.symbol.symbol.decode_symbol_kind(reader)
    range_ = destack._generated.source.file.model.span.decode_span(reader)
    selection_range = destack._generated.source.file.model.span.decode_span(reader)
    children = [decode_symbol(reader) for _ in range(reader.read_number())]

    return Symbol(
        name=name,
        detail=detail,
        kind=kind,
        range=range_,
        selection_range=selection_range,
        children=children,
    )


def to_json_symbol(value: Symbol) -> Json:
    """Return one JSON value for one Symbol."""
    return {
        "name": value.name,
        **({} if value.detail is None else {"detail": value.detail}),
        "kind": destack._generated.dir.symbol.symbol.to_json_symbol_kind(value.kind),
        "range": destack._generated.source.file.model.span.to_json_span(value.range),
        "selectionRange": destack._generated.source.file.model.span.to_json_span(
            value.selection_range
        ),
        "children": [to_json_symbol(item_0) for item_0 in value.children],
    }


def from_json_symbol(value: Json) -> Symbol:
    """Return one Symbol from one JSON value."""
    object_ = json_object(value)

    return Symbol(
        name=json_string(json_field(object_, "name")),
        detail=json_optional(object_, "detail", lambda value: json_string(value)),
        kind=destack._generated.dir.symbol.symbol.from_json_symbol_kind(
            json_field(object_, "kind")
        ),
        range=destack._generated.source.file.model.span.from_json_span(
            json_field(object_, "range")
        ),
        selection_range=destack._generated.source.file.model.span.from_json_span(
            json_field(object_, "selectionRange")
        ),
        children=[
            from_json_symbol(item_0)
            for item_0 in json_array(json_field(object_, "children"))
        ],
    )


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
