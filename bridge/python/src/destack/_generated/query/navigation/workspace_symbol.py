# generated bridge target, do not edit

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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_symbols_request(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> WorkspaceSymbolsRequest:
        """Decode one WorkspaceSymbolsRequest."""
        return decode_workspace_symbols_request(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_symbols_request(self)

    @classmethod
    def from_json(cls, value: Json) -> WorkspaceSymbolsRequest:
        """Return one WorkspaceSymbolsRequest from one JSON value."""
        return from_json_workspace_symbols_request(value)


def encode_workspace_symbols_request(
    writer: BinaryWriter, value: WorkspaceSymbolsRequest
) -> None:
    """Encode one WorkspaceSymbolsRequest."""
    writer.write_unsigned(len(value.profile_ids))
    for item_value_profile_ids_0 in value.profile_ids:
        destack._generated.source.file.model.profile.encode_profile_id(
            writer, item_value_profile_ids_0
        )
    writer.write_string(value.query)
    writer.write_unsigned(value.max_results)


def decode_workspace_symbols_request(reader: BinaryReader) -> WorkspaceSymbolsRequest:
    """Decode one WorkspaceSymbolsRequest."""
    profile_ids = [
        destack._generated.source.file.model.profile.decode_profile_id(reader)
        for _ in range(reader.read_number())
    ]
    query = reader.read_string()
    max_results = reader.read_number()

    return WorkspaceSymbolsRequest(
        profile_ids=profile_ids,
        query=query,
        max_results=max_results,
    )


def to_json_workspace_symbols_request(value: WorkspaceSymbolsRequest) -> Json:
    """Return one JSON value for one WorkspaceSymbolsRequest."""
    return {
        "profileIds": [
            destack._generated.source.file.model.profile.to_json_profile_id(item_0)
            for item_0 in value.profile_ids
        ],
        "query": value.query,
        "maxResults": value.max_results,
    }


def from_json_workspace_symbols_request(value: Json) -> WorkspaceSymbolsRequest:
    """Return one WorkspaceSymbolsRequest from one JSON value."""
    object_ = json_object(value)

    return WorkspaceSymbolsRequest(
        profile_ids=[
            destack._generated.source.file.model.profile.from_json_profile_id(item_0)
            for item_0 in json_array(json_field(object_, "profileIds"))
        ],
        query=json_string(json_field(object_, "query")),
        max_results=json_int(json_field(object_, "maxResults")),
    )


@dataclass(frozen=True, slots=True)
class WorkspaceSymbolsResponse:
    """Response payload for workspace symbols queries."""

    # workspace symbols
    symbols: Sequence[WorkspaceSymbol]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_symbols_response(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> WorkspaceSymbolsResponse:
        """Decode one WorkspaceSymbolsResponse."""
        return decode_workspace_symbols_response(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_symbols_response(self)

    @classmethod
    def from_json(cls, value: Json) -> WorkspaceSymbolsResponse:
        """Return one WorkspaceSymbolsResponse from one JSON value."""
        return from_json_workspace_symbols_response(value)


def encode_workspace_symbols_response(
    writer: BinaryWriter, value: WorkspaceSymbolsResponse
) -> None:
    """Encode one WorkspaceSymbolsResponse."""
    writer.write_unsigned(len(value.symbols))
    for item_value_symbols_0 in value.symbols:
        encode_workspace_symbol(writer, item_value_symbols_0)


def decode_workspace_symbols_response(reader: BinaryReader) -> WorkspaceSymbolsResponse:
    """Decode one WorkspaceSymbolsResponse."""
    symbols = [decode_workspace_symbol(reader) for _ in range(reader.read_number())]

    return WorkspaceSymbolsResponse(
        symbols=symbols,
    )


def to_json_workspace_symbols_response(value: WorkspaceSymbolsResponse) -> Json:
    """Return one JSON value for one WorkspaceSymbolsResponse."""
    return {
        "symbols": [to_json_workspace_symbol(item_0) for item_0 in value.symbols],
    }


def from_json_workspace_symbols_response(value: Json) -> WorkspaceSymbolsResponse:
    """Return one WorkspaceSymbolsResponse from one JSON value."""
    object_ = json_object(value)

    return WorkspaceSymbolsResponse(
        symbols=[
            from_json_workspace_symbol(item_0)
            for item_0 in json_array(json_field(object_, "symbols"))
        ],
    )


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_symbol(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> WorkspaceSymbol:
        """Decode one WorkspaceSymbol."""
        return decode_workspace_symbol(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_symbol(self)

    @classmethod
    def from_json(cls, value: Json) -> WorkspaceSymbol:
        """Return one WorkspaceSymbol from one JSON value."""
        return from_json_workspace_symbol(value)


def encode_workspace_symbol(writer: BinaryWriter, value: WorkspaceSymbol) -> None:
    """Encode one WorkspaceSymbol."""
    writer.write_string(value.name)
    destack._generated.query.dir.kind.encode_symbol_kind(writer, value.kind)
    destack._generated.query.core.target.encode_query_target(writer, value.target)
    if value.container is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.container)


def decode_workspace_symbol(reader: BinaryReader) -> WorkspaceSymbol:
    """Decode one WorkspaceSymbol."""
    name = reader.read_string()
    kind = destack._generated.query.dir.kind.decode_symbol_kind(reader)
    target = destack._generated.query.core.target.decode_query_target(reader)
    container = reader.read_option(lambda: reader.read_string())

    return WorkspaceSymbol(
        name=name,
        kind=kind,
        target=target,
        container=container,
    )


def to_json_workspace_symbol(value: WorkspaceSymbol) -> Json:
    """Return one JSON value for one WorkspaceSymbol."""
    return {
        "name": value.name,
        "kind": destack._generated.query.dir.kind.to_json_symbol_kind(value.kind),
        "target": destack._generated.query.core.target.to_json_query_target(
            value.target
        ),
        **({} if value.container is None else {"container": value.container}),
    }


def from_json_workspace_symbol(value: Json) -> WorkspaceSymbol:
    """Return one WorkspaceSymbol from one JSON value."""
    object_ = json_object(value)

    return WorkspaceSymbol(
        name=json_string(json_field(object_, "name")),
        kind=destack._generated.query.dir.kind.from_json_symbol_kind(
            json_field(object_, "kind")
        ),
        target=destack._generated.query.core.target.from_json_query_target(
            json_field(object_, "target")
        ),
        container=json_optional(object_, "container", lambda value: json_string(value)),
    )


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
