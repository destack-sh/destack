# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes

import destack._generated.protocol.query.core.target
import destack._generated.protocol.query.dir.kind
import destack._generated.protocol.source.file.model.profile

if TYPE_CHECKING:
    from destack._generated.protocol.query.core.target import (
        QueryTarget,
    )

    from destack._generated.protocol.query.dir.kind import (
        SymbolKind,
    )

    from destack._generated.protocol.source.file.model.profile import (
        ProfileId,
    )


@dataclass(frozen=True, slots=True)
class WorkspaceSymbolsRequest:
    """Request workspace symbols for a query string."""

    """The profiles to search."""
    profile_ids: Sequence[ProfileId]
    """The search query string."""
    query: str
    """The maximum number of results."""
    max_results: int


def encode_workspace_symbols_request(
    writer: Writer, value: WorkspaceSymbolsRequest
) -> None:
    writer.write_unsigned(len(value.profile_ids))
    for item_0 in value.profile_ids:
        destack._generated.protocol.source.file.model.profile.encode_profile_id(
            writer, item_0
        )
    writer.write_string(value.query)
    writer.write_unsigned(value.max_results)


def decode_workspace_symbols_request(reader: Reader) -> WorkspaceSymbolsRequest:
    field_0 = [
        destack._generated.protocol.source.file.model.profile.decode_profile_id(reader)
        for _ in range(reader.read_number())
    ]
    field_1 = reader.read_string()
    field_2 = reader.read_number()

    return WorkspaceSymbolsRequest(
        profile_ids=field_0,
        query=field_1,
        max_results=field_2,
    )


@dataclass(frozen=True, slots=True)
class WorkspaceSymbolsResponse:
    """Response payload for workspace symbols queries."""

    """Workspace symbols."""
    symbols: Sequence[WorkspaceSymbol]


def encode_workspace_symbols_response(
    writer: Writer, value: WorkspaceSymbolsResponse
) -> None:
    writer.write_unsigned(len(value.symbols))
    for item_0 in value.symbols:
        encode_workspace_symbol(writer, item_0)


def decode_workspace_symbols_response(reader: Reader) -> WorkspaceSymbolsResponse:
    field_0 = [decode_workspace_symbol(reader) for _ in range(reader.read_number())]

    return WorkspaceSymbolsResponse(
        symbols=field_0,
    )


@dataclass(frozen=True, slots=True)
class WorkspaceSymbol:
    """A symbol in the workspace (flat list for workspace symbol search)."""

    """The symbol's name."""
    name: str
    """The kind of symbol."""
    kind: SymbolKind
    """The symbol source target."""
    target: QueryTarget
    """Container name (e.g., class name for methods)."""
    container: str | None


def encode_workspace_symbol(writer: Writer, value: WorkspaceSymbol) -> None:
    writer.write_string(value.name)
    destack._generated.protocol.query.dir.kind.encode_symbol_kind(writer, value.kind)
    destack._generated.protocol.query.core.target.encode_query_target(
        writer, value.target
    )
    if value.container is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.container)


def decode_workspace_symbol(reader: Reader) -> WorkspaceSymbol:
    field_0 = reader.read_string()
    field_1 = destack._generated.protocol.query.dir.kind.decode_symbol_kind(reader)
    field_2 = destack._generated.protocol.query.core.target.decode_query_target(reader)
    field_3 = reader.read_option(lambda: reader.read_string())

    return WorkspaceSymbol(
        name=field_0,
        kind=field_1,
        target=field_2,
        container=field_3,
    )


__all__ = [
    "WorkspaceSymbolsRequest",
    "encode_workspace_symbols_request",
    "decode_workspace_symbols_request",
    "WorkspaceSymbolsResponse",
    "encode_workspace_symbols_response",
    "decode_workspace_symbols_response",
    "WorkspaceSymbol",
    "encode_workspace_symbol",
    "decode_workspace_symbol",
]
