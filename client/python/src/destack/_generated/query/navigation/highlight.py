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

import destack._generated.query.protocol.target
import destack._generated.source.file.model.span


@dataclass(frozen=True, slots=True)
class HighlightRequest:
    """Request highlights at a cursor position."""

    # the queried position
    position: destack._generated.query.protocol.target.Position

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_highlight_request(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> HighlightRequest:
        """Decode one HighlightRequest."""
        return decode_highlight_request(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_highlight_request(self)

    @classmethod
    def from_json(cls, value: Json) -> HighlightRequest:
        """Return one HighlightRequest from one JSON value."""
        return from_json_highlight_request(value)


def encode_highlight_request(writer: BinaryWriter, value: HighlightRequest) -> None:
    """Encode one HighlightRequest."""
    destack._generated.query.protocol.target.encode_position(writer, value.position)


def decode_highlight_request(reader: BinaryReader) -> HighlightRequest:
    """Decode one HighlightRequest."""
    position = destack._generated.query.protocol.target.decode_position(reader)

    return HighlightRequest(
        position=position,
    )


def to_json_highlight_request(value: HighlightRequest) -> Json:
    """Return one JSON value for one HighlightRequest."""
    return {
        "position": destack._generated.query.protocol.target.to_json_position(
            value.position
        ),
    }


def from_json_highlight_request(value: Json) -> HighlightRequest:
    """Return one HighlightRequest from one JSON value."""
    object_ = json_object(value)

    return HighlightRequest(
        position=destack._generated.query.protocol.target.from_json_position(
            json_field(object_, "position")
        ),
    )


@dataclass(frozen=True, slots=True)
class HighlightResponse:
    """Response payload for highlight queries."""

    # highlights
    highlights: Sequence[Highlight]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_highlight_response(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> HighlightResponse:
        """Decode one HighlightResponse."""
        return decode_highlight_response(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_highlight_response(self)

    @classmethod
    def from_json(cls, value: Json) -> HighlightResponse:
        """Return one HighlightResponse from one JSON value."""
        return from_json_highlight_response(value)


def encode_highlight_response(writer: BinaryWriter, value: HighlightResponse) -> None:
    """Encode one HighlightResponse."""
    writer.write_unsigned(len(value.highlights))
    for item_value_highlights_0 in value.highlights:
        encode_highlight(writer, item_value_highlights_0)


def decode_highlight_response(reader: BinaryReader) -> HighlightResponse:
    """Decode one HighlightResponse."""
    highlights = [decode_highlight(reader) for _ in range(reader.read_number())]

    return HighlightResponse(
        highlights=highlights,
    )


def to_json_highlight_response(value: HighlightResponse) -> Json:
    """Return one JSON value for one HighlightResponse."""
    return {
        "highlights": [to_json_highlight(item_0) for item_0 in value.highlights],
    }


def from_json_highlight_response(value: Json) -> HighlightResponse:
    """Return one HighlightResponse from one JSON value."""
    object_ = json_object(value)

    return HighlightResponse(
        highlights=[
            from_json_highlight(item_0)
            for item_0 in json_array(json_field(object_, "highlights"))
        ],
    )


@dataclass(frozen=True, slots=True)
class Highlight:
    """A highlighted range in a module."""

    # the highlighted range
    range: destack._generated.source.file.model.span.Span
    # the kind of highlight
    kind: HighlightKind

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_highlight(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Highlight:
        """Decode one Highlight."""
        return decode_highlight(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_highlight(self)

    @classmethod
    def from_json(cls, value: Json) -> Highlight:
        """Return one Highlight from one JSON value."""
        return from_json_highlight(value)


def encode_highlight(writer: BinaryWriter, value: Highlight) -> None:
    """Encode one Highlight."""
    destack._generated.source.file.model.span.encode_span(writer, value.range)
    encode_highlight_kind(writer, value.kind)


def decode_highlight(reader: BinaryReader) -> Highlight:
    """Decode one Highlight."""
    range_ = destack._generated.source.file.model.span.decode_span(reader)
    kind = decode_highlight_kind(reader)

    return Highlight(
        range=range_,
        kind=kind,
    )


def to_json_highlight(value: Highlight) -> Json:
    """Return one JSON value for one Highlight."""
    return {
        "range": destack._generated.source.file.model.span.to_json_span(value.range),
        "kind": to_json_highlight_kind(value.kind),
    }


def from_json_highlight(value: Json) -> Highlight:
    """Return one Highlight from one JSON value."""
    object_ = json_object(value)

    return Highlight(
        range=destack._generated.source.file.model.span.from_json_span(
            json_field(object_, "range")
        ),
        kind=from_json_highlight_kind(json_field(object_, "kind")),
    )


"""Kind of highlight."""
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
    "HighlightRequest",
    "encode_highlight_request",
    "decode_highlight_request",
    "to_json_highlight_request",
    "from_json_highlight_request",
    "HighlightResponse",
    "encode_highlight_response",
    "decode_highlight_response",
    "to_json_highlight_response",
    "from_json_highlight_response",
    "Highlight",
    "encode_highlight",
    "decode_highlight",
    "to_json_highlight",
    "from_json_highlight",
    "HighlightKind",
    "encode_highlight_kind",
    "decode_highlight_kind",
    "to_json_highlight_kind",
    "from_json_highlight_kind",
]
