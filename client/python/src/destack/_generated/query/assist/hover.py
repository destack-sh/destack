# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_field,
    json_object,
    json_optional,
    json_string,
)

import destack._generated.query.core.target
import destack._generated.source.file.model.span


@dataclass(frozen=True, slots=True)
class HoverRequest:
    """Request hover information at a cursor position."""

    # the queried position
    position: destack._generated.query.core.target.QueryPosition

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_hover_request(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> HoverRequest:
        """Decode one HoverRequest."""
        return decode_hover_request(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_hover_request(self)

    @classmethod
    def from_json(cls, value: Json) -> HoverRequest:
        """Return one HoverRequest from one JSON value."""
        return from_json_hover_request(value)


def encode_hover_request(writer: BinaryWriter, value: HoverRequest) -> None:
    """Encode one HoverRequest."""
    destack._generated.query.core.target.encode_query_position(writer, value.position)


def decode_hover_request(reader: BinaryReader) -> HoverRequest:
    """Decode one HoverRequest."""
    position = destack._generated.query.core.target.decode_query_position(reader)

    return HoverRequest(
        position=position,
    )


def to_json_hover_request(value: HoverRequest) -> Json:
    """Return one JSON value for one HoverRequest."""
    return {
        "position": destack._generated.query.core.target.to_json_query_position(
            value.position
        ),
    }


def from_json_hover_request(value: Json) -> HoverRequest:
    """Return one HoverRequest from one JSON value."""
    object_ = json_object(value)

    return HoverRequest(
        position=destack._generated.query.core.target.from_json_query_position(
            json_field(object_, "position")
        ),
    )


@dataclass(frozen=True, slots=True)
class HoverResponse:
    """Response payload for hover queries."""

    # hover information, if available
    hover: Hover | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_hover_response(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> HoverResponse:
        """Decode one HoverResponse."""
        return decode_hover_response(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_hover_response(self)

    @classmethod
    def from_json(cls, value: Json) -> HoverResponse:
        """Return one HoverResponse from one JSON value."""
        return from_json_hover_response(value)


def encode_hover_response(writer: BinaryWriter, value: HoverResponse) -> None:
    """Encode one HoverResponse."""
    if value.hover is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_hover(writer, value.hover)


def decode_hover_response(reader: BinaryReader) -> HoverResponse:
    """Decode one HoverResponse."""
    hover = reader.read_option(lambda: decode_hover(reader))

    return HoverResponse(
        hover=hover,
    )


def to_json_hover_response(value: HoverResponse) -> Json:
    """Return one JSON value for one HoverResponse."""
    return {
        **({} if value.hover is None else {"hover": to_json_hover(value.hover)}),
    }


def from_json_hover_response(value: Json) -> HoverResponse:
    """Return one HoverResponse from one JSON value."""
    object_ = json_object(value)

    return HoverResponse(
        hover=json_optional(object_, "hover", lambda value: from_json_hover(value)),
    )


@dataclass(frozen=True, slots=True)
class Hover:
    """Hover payload for a source position."""

    # the type/signature in code format
    signature: str
    # documentation (markdown)
    documentation: str | None
    # resolved type information when available
    type_text: str | None
    # source location text when available
    location: str | None
    # the range of the hovered element
    range: destack._generated.source.file.model.span.Span | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_hover(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Hover:
        """Decode one Hover."""
        return decode_hover(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_hover(self)

    @classmethod
    def from_json(cls, value: Json) -> Hover:
        """Return one Hover from one JSON value."""
        return from_json_hover(value)


def encode_hover(writer: BinaryWriter, value: Hover) -> None:
    """Encode one Hover."""
    writer.write_string(value.signature)
    if value.documentation is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.documentation)
    if value.type_text is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.type_text)
    if value.location is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.location)
    if value.range is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.source.file.model.span.encode_span(writer, value.range)


def decode_hover(reader: BinaryReader) -> Hover:
    """Decode one Hover."""
    signature = reader.read_string()
    documentation = reader.read_option(lambda: reader.read_string())
    type_text = reader.read_option(lambda: reader.read_string())
    location = reader.read_option(lambda: reader.read_string())
    range_ = reader.read_option(
        lambda: destack._generated.source.file.model.span.decode_span(reader)
    )

    return Hover(
        signature=signature,
        documentation=documentation,
        type_text=type_text,
        location=location,
        range=range_,
    )


def to_json_hover(value: Hover) -> Json:
    """Return one JSON value for one Hover."""
    return {
        "signature": value.signature,
        **(
            {}
            if value.documentation is None
            else {"documentation": value.documentation}
        ),
        **({} if value.type_text is None else {"typeText": value.type_text}),
        **({} if value.location is None else {"location": value.location}),
        **(
            {}
            if value.range is None
            else {
                "range": destack._generated.source.file.model.span.to_json_span(
                    value.range
                )
            }
        ),
    }


def from_json_hover(value: Json) -> Hover:
    """Return one Hover from one JSON value."""
    object_ = json_object(value)

    return Hover(
        signature=json_string(json_field(object_, "signature")),
        documentation=json_optional(
            object_, "documentation", lambda value: json_string(value)
        ),
        type_text=json_optional(object_, "typeText", lambda value: json_string(value)),
        location=json_optional(object_, "location", lambda value: json_string(value)),
        range=json_optional(
            object_,
            "range",
            lambda value: destack._generated.source.file.model.span.from_json_span(
                value
            ),
        ),
    )


__all__ = [
    "HoverRequest",
    "encode_hover_request",
    "decode_hover_request",
    "to_json_hover_request",
    "from_json_hover_request",
    "HoverResponse",
    "encode_hover_response",
    "decode_hover_response",
    "to_json_hover_response",
    "from_json_hover_response",
    "Hover",
    "encode_hover",
    "decode_hover",
    "to_json_hover",
    "from_json_hover",
]
