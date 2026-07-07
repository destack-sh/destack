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
    json_int,
    json_object,
    json_optional,
    json_string,
)

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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_symbol_search_request(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> SymbolSearchRequest:
        """Decode one SymbolSearchRequest."""
        return decode_symbol_search_request(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_symbol_search_request(self)

    @classmethod
    def from_json(cls, value: Json) -> SymbolSearchRequest:
        """Return one SymbolSearchRequest from one JSON value."""
        return from_json_symbol_search_request(value)


def encode_symbol_search_request(
    writer: BinaryWriter, value: SymbolSearchRequest
) -> None:
    """Encode one SymbolSearchRequest."""
    writer.write_unsigned(len(value.profile_ids))
    for item_value_profile_ids_0 in value.profile_ids:
        destack._generated.source.file.model.profile.encode_profile_id(
            writer, item_value_profile_ids_0
        )
    writer.write_string(value.query)
    writer.write_unsigned(value.max_results)


def decode_symbol_search_request(reader: BinaryReader) -> SymbolSearchRequest:
    """Decode one SymbolSearchRequest."""
    profile_ids = [
        destack._generated.source.file.model.profile.decode_profile_id(reader)
        for _ in range(reader.read_number())
    ]
    query = reader.read_string()
    max_results = reader.read_number()

    return SymbolSearchRequest(
        profile_ids=profile_ids,
        query=query,
        max_results=max_results,
    )


def to_json_symbol_search_request(value: SymbolSearchRequest) -> Json:
    """Return one JSON value for one SymbolSearchRequest."""
    return {
        "profileIds": [
            destack._generated.source.file.model.profile.to_json_profile_id(item_0)
            for item_0 in value.profile_ids
        ],
        "query": value.query,
        "maxResults": value.max_results,
    }


def from_json_symbol_search_request(value: Json) -> SymbolSearchRequest:
    """Return one SymbolSearchRequest from one JSON value."""
    object_ = json_object(value)

    return SymbolSearchRequest(
        profile_ids=[
            destack._generated.source.file.model.profile.from_json_profile_id(item_0)
            for item_0 in json_array(json_field(object_, "profileIds"))
        ],
        query=json_string(json_field(object_, "query")),
        max_results=json_int(json_field(object_, "maxResults")),
    )


@dataclass(frozen=True, slots=True)
class SymbolSearchResponse:
    """Response payload for symbol search queries."""

    # the matching symbols
    symbols: Sequence[SymbolMatch]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_symbol_search_response(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> SymbolSearchResponse:
        """Decode one SymbolSearchResponse."""
        return decode_symbol_search_response(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_symbol_search_response(self)

    @classmethod
    def from_json(cls, value: Json) -> SymbolSearchResponse:
        """Return one SymbolSearchResponse from one JSON value."""
        return from_json_symbol_search_response(value)


def encode_symbol_search_response(
    writer: BinaryWriter, value: SymbolSearchResponse
) -> None:
    """Encode one SymbolSearchResponse."""
    writer.write_unsigned(len(value.symbols))
    for item_value_symbols_0 in value.symbols:
        encode_symbol_match(writer, item_value_symbols_0)


def decode_symbol_search_response(reader: BinaryReader) -> SymbolSearchResponse:
    """Decode one SymbolSearchResponse."""
    symbols = [decode_symbol_match(reader) for _ in range(reader.read_number())]

    return SymbolSearchResponse(
        symbols=symbols,
    )


def to_json_symbol_search_response(value: SymbolSearchResponse) -> Json:
    """Return one JSON value for one SymbolSearchResponse."""
    return {
        "symbols": [to_json_symbol_match(item_0) for item_0 in value.symbols],
    }


def from_json_symbol_search_response(value: Json) -> SymbolSearchResponse:
    """Return one SymbolSearchResponse from one JSON value."""
    object_ = json_object(value)

    return SymbolSearchResponse(
        symbols=[
            from_json_symbol_match(item_0)
            for item_0 in json_array(json_field(object_, "symbols"))
        ],
    )


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_symbol_match(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> SymbolMatch:
        """Decode one SymbolMatch."""
        return decode_symbol_match(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_symbol_match(self)

    @classmethod
    def from_json(cls, value: Json) -> SymbolMatch:
        """Return one SymbolMatch from one JSON value."""
        return from_json_symbol_match(value)


def encode_symbol_match(writer: BinaryWriter, value: SymbolMatch) -> None:
    """Encode one SymbolMatch."""
    writer.write_string(value.name)
    destack._generated.dir.symbol.symbol.encode_symbol_kind(writer, value.kind)
    destack._generated.query.protocol.target.encode_target(writer, value.target)
    if value.container is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.container)


def decode_symbol_match(reader: BinaryReader) -> SymbolMatch:
    """Decode one SymbolMatch."""
    name = reader.read_string()
    kind = destack._generated.dir.symbol.symbol.decode_symbol_kind(reader)
    target = destack._generated.query.protocol.target.decode_target(reader)
    container = reader.read_option(lambda: reader.read_string())

    return SymbolMatch(
        name=name,
        kind=kind,
        target=target,
        container=container,
    )


def to_json_symbol_match(value: SymbolMatch) -> Json:
    """Return one JSON value for one SymbolMatch."""
    return {
        "name": value.name,
        "kind": destack._generated.dir.symbol.symbol.to_json_symbol_kind(value.kind),
        "target": destack._generated.query.protocol.target.to_json_target(value.target),
        **({} if value.container is None else {"container": value.container}),
    }


def from_json_symbol_match(value: Json) -> SymbolMatch:
    """Return one SymbolMatch from one JSON value."""
    object_ = json_object(value)

    return SymbolMatch(
        name=json_string(json_field(object_, "name")),
        kind=destack._generated.dir.symbol.symbol.from_json_symbol_kind(
            json_field(object_, "kind")
        ),
        target=destack._generated.query.protocol.target.from_json_target(
            json_field(object_, "target")
        ),
        container=json_optional(object_, "container", lambda value: json_string(value)),
    )


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
