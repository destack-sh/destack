# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes

import destack._generated.protocol.query.core.target
import destack._generated.protocol.source.file.model.span

if TYPE_CHECKING:
    from destack._generated.protocol.query.core.target import (
        QueryPosition,
    )

    from destack._generated.protocol.source.file.model.span import (
        Span,
    )


@dataclass(frozen=True, slots=True)
class DocumentHighlightRequest:
    """Request highlights at a cursor position."""

    """The queried position."""
    position: QueryPosition


def encode_document_highlight_request(
    writer: Writer, value: DocumentHighlightRequest
) -> None:
    destack._generated.protocol.query.core.target.encode_query_position(
        writer, value.position
    )


def decode_document_highlight_request(reader: Reader) -> DocumentHighlightRequest:
    field_0 = destack._generated.protocol.query.core.target.decode_query_position(
        reader
    )

    return DocumentHighlightRequest(
        position=field_0,
    )


@dataclass(frozen=True, slots=True)
class DocumentHighlightResponse:
    """Response payload for document highlight queries."""

    """Document highlights."""
    highlights: Sequence[DocumentHighlight]


def encode_document_highlight_response(
    writer: Writer, value: DocumentHighlightResponse
) -> None:
    writer.write_unsigned(len(value.highlights))
    for item_0 in value.highlights:
        encode_document_highlight(writer, item_0)


def decode_document_highlight_response(reader: Reader) -> DocumentHighlightResponse:
    field_0 = [decode_document_highlight(reader) for _ in range(reader.read_number())]

    return DocumentHighlightResponse(
        highlights=field_0,
    )


@dataclass(frozen=True, slots=True)
class DocumentHighlight:
    """A highlighted range in a document."""

    """The highlighted range."""
    range: Span
    """The kind of highlight."""
    kind: HighlightKind


def encode_document_highlight(writer: Writer, value: DocumentHighlight) -> None:
    destack._generated.protocol.source.file.model.span.encode_span(writer, value.range)
    encode_highlight_kind(writer, value.kind)


def decode_document_highlight(reader: Reader) -> DocumentHighlight:
    field_0 = destack._generated.protocol.source.file.model.span.decode_span(reader)
    field_1 = decode_highlight_kind(reader)

    return DocumentHighlight(
        range=field_0,
        kind=field_1,
    )


"""Kind of document highlight."""
HighlightKind: TypeAlias = Literal["text"] | Literal["read"] | Literal["write"]


def encode_highlight_kind(writer: Writer, value: HighlightKind) -> None:
    if value == "text":
        writer.write_unsigned(0)
    elif value == "read":
        writer.write_unsigned(1)
    elif value == "write":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_highlight_kind(reader: Reader) -> HighlightKind:
    variant = reader.read_number()

    if variant == 0:
        return "text"
    elif variant == 1:
        return "read"
    elif variant == 2:
        return "write"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


__all__ = [
    "DocumentHighlightRequest",
    "encode_document_highlight_request",
    "decode_document_highlight_request",
    "DocumentHighlightResponse",
    "encode_document_highlight_response",
    "decode_document_highlight_response",
    "DocumentHighlight",
    "encode_document_highlight",
    "decode_document_highlight",
    "HighlightKind",
    "encode_highlight_kind",
    "decode_highlight_kind",
]
