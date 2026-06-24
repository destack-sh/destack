# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_array,
    json_field,
    json_object,
    json_string,
)

import destack._generated.query.core.target
import destack._generated.source.file.model.span


@dataclass(frozen=True, slots=True)
class DocumentHighlightRequest:
    """Request highlights at a cursor position."""

    # the queried position
    position: destack._generated.query.core.target.QueryPosition

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_document_highlight_request(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> DocumentHighlightRequest:
        """Decode one DocumentHighlightRequest."""
        return decode_document_highlight_request(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_document_highlight_request(self)

    @classmethod
    def from_json(cls, value: Json) -> DocumentHighlightRequest:
        """Return one DocumentHighlightRequest from one JSON value."""
        return from_json_document_highlight_request(value)


def encode_document_highlight_request(
    writer: BinaryWriter, value: DocumentHighlightRequest
) -> None:
    """Encode one DocumentHighlightRequest."""
    destack._generated.query.core.target.encode_query_position(writer, value.position)


def decode_document_highlight_request(reader: BinaryReader) -> DocumentHighlightRequest:
    """Decode one DocumentHighlightRequest."""
    position = destack._generated.query.core.target.decode_query_position(reader)

    return DocumentHighlightRequest(
        position=position,
    )


def to_json_document_highlight_request(value: DocumentHighlightRequest) -> Json:
    """Return one JSON value for one DocumentHighlightRequest."""
    return {
        "position": destack._generated.query.core.target.to_json_query_position(
            value.position
        ),
    }


def from_json_document_highlight_request(value: Json) -> DocumentHighlightRequest:
    """Return one DocumentHighlightRequest from one JSON value."""
    object_ = json_object(value)

    return DocumentHighlightRequest(
        position=destack._generated.query.core.target.from_json_query_position(
            json_field(object_, "position")
        ),
    )


@dataclass(frozen=True, slots=True)
class DocumentHighlightResponse:
    """Response payload for document highlight queries."""

    # document highlights
    highlights: Sequence[DocumentHighlight]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_document_highlight_response(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> DocumentHighlightResponse:
        """Decode one DocumentHighlightResponse."""
        return decode_document_highlight_response(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_document_highlight_response(self)

    @classmethod
    def from_json(cls, value: Json) -> DocumentHighlightResponse:
        """Return one DocumentHighlightResponse from one JSON value."""
        return from_json_document_highlight_response(value)


def encode_document_highlight_response(
    writer: BinaryWriter, value: DocumentHighlightResponse
) -> None:
    """Encode one DocumentHighlightResponse."""
    writer.write_unsigned(len(value.highlights))
    for item_value_highlights_0 in value.highlights:
        encode_document_highlight(writer, item_value_highlights_0)


def decode_document_highlight_response(
    reader: BinaryReader,
) -> DocumentHighlightResponse:
    """Decode one DocumentHighlightResponse."""
    highlights = [
        decode_document_highlight(reader) for _ in range(reader.read_number())
    ]

    return DocumentHighlightResponse(
        highlights=highlights,
    )


def to_json_document_highlight_response(value: DocumentHighlightResponse) -> Json:
    """Return one JSON value for one DocumentHighlightResponse."""
    return {
        "highlights": [
            to_json_document_highlight(item_0) for item_0 in value.highlights
        ],
    }


def from_json_document_highlight_response(value: Json) -> DocumentHighlightResponse:
    """Return one DocumentHighlightResponse from one JSON value."""
    object_ = json_object(value)

    return DocumentHighlightResponse(
        highlights=[
            from_json_document_highlight(item_0)
            for item_0 in json_array(json_field(object_, "highlights"))
        ],
    )


@dataclass(frozen=True, slots=True)
class DocumentHighlight:
    """A highlighted range in a document."""

    # the highlighted range
    range: destack._generated.source.file.model.span.Span
    # the kind of highlight
    kind: HighlightKind

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_document_highlight(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> DocumentHighlight:
        """Decode one DocumentHighlight."""
        return decode_document_highlight(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_document_highlight(self)

    @classmethod
    def from_json(cls, value: Json) -> DocumentHighlight:
        """Return one DocumentHighlight from one JSON value."""
        return from_json_document_highlight(value)


def encode_document_highlight(writer: BinaryWriter, value: DocumentHighlight) -> None:
    """Encode one DocumentHighlight."""
    destack._generated.source.file.model.span.encode_span(writer, value.range)
    encode_highlight_kind(writer, value.kind)


def decode_document_highlight(reader: BinaryReader) -> DocumentHighlight:
    """Decode one DocumentHighlight."""
    range_ = destack._generated.source.file.model.span.decode_span(reader)
    kind = decode_highlight_kind(reader)

    return DocumentHighlight(
        range=range_,
        kind=kind,
    )


def to_json_document_highlight(value: DocumentHighlight) -> Json:
    """Return one JSON value for one DocumentHighlight."""
    return {
        "range": destack._generated.source.file.model.span.to_json_span(value.range),
        "kind": to_json_highlight_kind(value.kind),
    }


def from_json_document_highlight(value: Json) -> DocumentHighlight:
    """Return one DocumentHighlight from one JSON value."""
    object_ = json_object(value)

    return DocumentHighlight(
        range=destack._generated.source.file.model.span.from_json_span(
            json_field(object_, "range")
        ),
        kind=from_json_highlight_kind(json_field(object_, "kind")),
    )


"""Kind of document highlight."""
HighlightKind: typing.TypeAlias = (
    typing.Literal["text"] | typing.Literal["read"] | typing.Literal["write"]
)


def encode_highlight_kind(writer: BinaryWriter, value: HighlightKind) -> None:
    """Encode one HighlightKind."""
    if value == "text":
        writer.write_unsigned(0)
    elif value == "read":
        writer.write_unsigned(1)
    elif value == "write":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_highlight_kind(reader: BinaryReader) -> HighlightKind:
    """Decode one HighlightKind."""
    variant = reader.read_number()

    if variant == 0:
        return "text"
    elif variant == 1:
        return "read"
    elif variant == 2:
        return "write"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_highlight_kind(value: HighlightKind) -> Json:
    """Return one JSON value for one HighlightKind."""
    return value


def from_json_highlight_kind(value: Json) -> HighlightKind:
    """Return one HighlightKind from one JSON value."""
    variant = json_string(value)

    if variant == "text":
        return "text"
    elif variant == "read":
        return "read"
    elif variant == "write":
        return "write"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


__all__ = [
    "DocumentHighlightRequest",
    "encode_document_highlight_request",
    "decode_document_highlight_request",
    "to_json_document_highlight_request",
    "from_json_document_highlight_request",
    "DocumentHighlightResponse",
    "encode_document_highlight_response",
    "decode_document_highlight_response",
    "to_json_document_highlight_response",
    "from_json_document_highlight_response",
    "DocumentHighlight",
    "encode_document_highlight",
    "decode_document_highlight",
    "to_json_document_highlight",
    "from_json_document_highlight",
    "HighlightKind",
    "encode_highlight_kind",
    "decode_highlight_kind",
    "to_json_highlight_kind",
    "from_json_highlight_kind",
]
