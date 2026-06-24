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
import destack._generated.source.edit.edit


@dataclass(frozen=True, slots=True)
class ExtractVariableRequest:
    """Request payload for extract variable queries."""

    # the selected source range
    range: destack._generated.query.core.target.QueryRange
    # the name for the extracted variable
    new_name: str

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_extract_variable_request(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ExtractVariableRequest:
        """Decode one ExtractVariableRequest."""
        return decode_extract_variable_request(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_extract_variable_request(self)

    @classmethod
    def from_json(cls, value: Json) -> ExtractVariableRequest:
        """Return one ExtractVariableRequest from one JSON value."""
        return from_json_extract_variable_request(value)


def encode_extract_variable_request(
    writer: BinaryWriter, value: ExtractVariableRequest
) -> None:
    """Encode one ExtractVariableRequest."""
    destack._generated.query.core.target.encode_query_range(writer, value.range)
    writer.write_string(value.new_name)


def decode_extract_variable_request(reader: BinaryReader) -> ExtractVariableRequest:
    """Decode one ExtractVariableRequest."""
    range_ = destack._generated.query.core.target.decode_query_range(reader)
    new_name = reader.read_string()

    return ExtractVariableRequest(
        range=range_,
        new_name=new_name,
    )


def to_json_extract_variable_request(value: ExtractVariableRequest) -> Json:
    """Return one JSON value for one ExtractVariableRequest."""
    return {
        "range": destack._generated.query.core.target.to_json_query_range(value.range),
        "newName": value.new_name,
    }


def from_json_extract_variable_request(value: Json) -> ExtractVariableRequest:
    """Return one ExtractVariableRequest from one JSON value."""
    object_ = json_object(value)

    return ExtractVariableRequest(
        range=destack._generated.query.core.target.from_json_query_range(
            json_field(object_, "range")
        ),
        new_name=json_string(json_field(object_, "newName")),
    )


@dataclass(frozen=True, slots=True)
class ExtractVariableResponse:
    """Response payload for extract variable queries."""

    # extract variable edit, if available
    edit: destack._generated.source.edit.edit.PatchSet | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_extract_variable_response(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ExtractVariableResponse:
        """Decode one ExtractVariableResponse."""
        return decode_extract_variable_response(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_extract_variable_response(self)

    @classmethod
    def from_json(cls, value: Json) -> ExtractVariableResponse:
        """Return one ExtractVariableResponse from one JSON value."""
        return from_json_extract_variable_response(value)


def encode_extract_variable_response(
    writer: BinaryWriter, value: ExtractVariableResponse
) -> None:
    """Encode one ExtractVariableResponse."""
    if value.edit is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.source.edit.edit.encode_patch_set(writer, value.edit)


def decode_extract_variable_response(reader: BinaryReader) -> ExtractVariableResponse:
    """Decode one ExtractVariableResponse."""
    edit = reader.read_option(
        lambda: destack._generated.source.edit.edit.decode_patch_set(reader)
    )

    return ExtractVariableResponse(
        edit=edit,
    )


def to_json_extract_variable_response(value: ExtractVariableResponse) -> Json:
    """Return one JSON value for one ExtractVariableResponse."""
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


def from_json_extract_variable_response(value: Json) -> ExtractVariableResponse:
    """Return one ExtractVariableResponse from one JSON value."""
    object_ = json_object(value)

    return ExtractVariableResponse(
        edit=json_optional(
            object_,
            "edit",
            lambda value: destack._generated.source.edit.edit.from_json_patch_set(
                value
            ),
        ),
    )


__all__ = [
    "ExtractVariableRequest",
    "encode_extract_variable_request",
    "decode_extract_variable_request",
    "to_json_extract_variable_request",
    "from_json_extract_variable_request",
    "ExtractVariableResponse",
    "encode_extract_variable_response",
    "decode_extract_variable_response",
    "to_json_extract_variable_response",
    "from_json_extract_variable_response",
]
