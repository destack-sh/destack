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
)

import destack._generated.query.core.target
import destack._generated.source.edit.edit


@dataclass(frozen=True, slots=True)
class InlineRequest:
    """Request payload for inline refactor queries."""

    # the queried position
    position: destack._generated.query.core.target.QueryPosition

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_inline_request(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> InlineRequest:
        """Decode one InlineRequest."""
        return decode_inline_request(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_inline_request(self)

    @classmethod
    def from_json(cls, value: Json) -> InlineRequest:
        """Return one InlineRequest from one JSON value."""
        return from_json_inline_request(value)


def encode_inline_request(writer: BinaryWriter, value: InlineRequest) -> None:
    """Encode one InlineRequest."""
    destack._generated.query.core.target.encode_query_position(writer, value.position)


def decode_inline_request(reader: BinaryReader) -> InlineRequest:
    """Decode one InlineRequest."""
    position = destack._generated.query.core.target.decode_query_position(reader)

    return InlineRequest(
        position=position,
    )


def to_json_inline_request(value: InlineRequest) -> Json:
    """Return one JSON value for one InlineRequest."""
    return {
        "position": destack._generated.query.core.target.to_json_query_position(
            value.position
        ),
    }


def from_json_inline_request(value: Json) -> InlineRequest:
    """Return one InlineRequest from one JSON value."""
    object_ = json_object(value)

    return InlineRequest(
        position=destack._generated.query.core.target.from_json_query_position(
            json_field(object_, "position")
        ),
    )


@dataclass(frozen=True, slots=True)
class InlineResponse:
    """Response payload for inline refactor queries."""

    # inline edit, if available
    edit: destack._generated.source.edit.edit.PatchSet | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_inline_response(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> InlineResponse:
        """Decode one InlineResponse."""
        return decode_inline_response(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_inline_response(self)

    @classmethod
    def from_json(cls, value: Json) -> InlineResponse:
        """Return one InlineResponse from one JSON value."""
        return from_json_inline_response(value)


def encode_inline_response(writer: BinaryWriter, value: InlineResponse) -> None:
    """Encode one InlineResponse."""
    if value.edit is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.source.edit.edit.encode_patch_set(writer, value.edit)


def decode_inline_response(reader: BinaryReader) -> InlineResponse:
    """Decode one InlineResponse."""
    edit = reader.read_option(
        lambda: destack._generated.source.edit.edit.decode_patch_set(reader)
    )

    return InlineResponse(
        edit=edit,
    )


def to_json_inline_response(value: InlineResponse) -> Json:
    """Return one JSON value for one InlineResponse."""
    return {
        **(
            {}
            if value.edit is None
            else {
                "edit": destack._generated.source.edit.edit.to_json_patch_set(
                    value.edit
                )
            }
        ),
    }


def from_json_inline_response(value: Json) -> InlineResponse:
    """Return one InlineResponse from one JSON value."""
    object_ = json_object(value)

    return InlineResponse(
        edit=json_optional(
            object_,
            "edit",
            lambda value: destack._generated.source.edit.edit.from_json_patch_set(
                value
            ),
        ),
    )


__all__ = [
    "InlineRequest",
    "encode_inline_request",
    "decode_inline_request",
    "to_json_inline_request",
    "from_json_inline_request",
    "InlineResponse",
    "encode_inline_response",
    "decode_inline_response",
    "to_json_inline_response",
    "from_json_inline_response",
]
