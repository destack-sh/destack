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

import destack._generated.query.protocol.target
import destack._generated.source.edit.edit


@dataclass(frozen=True, slots=True)
class ChangeSignatureRequest:
    """Request payload for change signature queries."""

    # the queried position
    position: destack._generated.query.protocol.target.Position
    # the new parameter list, comma-separated
    new_parameters: str
    # the new argument list, comma-separated
    new_arguments: str

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_change_signature_request(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ChangeSignatureRequest:
        """Decode one ChangeSignatureRequest."""
        return decode_change_signature_request(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_change_signature_request(self)

    @classmethod
    def from_json(cls, value: Json) -> ChangeSignatureRequest:
        """Return one ChangeSignatureRequest from one JSON value."""
        return from_json_change_signature_request(value)


def encode_change_signature_request(
    writer: BinaryWriter, value: ChangeSignatureRequest
) -> None:
    """Encode one ChangeSignatureRequest."""
    destack._generated.query.protocol.target.encode_position(writer, value.position)
    writer.write_string(value.new_parameters)
    writer.write_string(value.new_arguments)


def decode_change_signature_request(reader: BinaryReader) -> ChangeSignatureRequest:
    """Decode one ChangeSignatureRequest."""
    position = destack._generated.query.protocol.target.decode_position(reader)
    new_parameters = reader.read_string()
    new_arguments = reader.read_string()

    return ChangeSignatureRequest(
        position=position,
        new_parameters=new_parameters,
        new_arguments=new_arguments,
    )


def to_json_change_signature_request(value: ChangeSignatureRequest) -> Json:
    """Return one JSON value for one ChangeSignatureRequest."""
    return {
        "position": destack._generated.query.protocol.target.to_json_position(
            value.position
        ),
        "newParameters": value.new_parameters,
        "newArguments": value.new_arguments,
    }


def from_json_change_signature_request(value: Json) -> ChangeSignatureRequest:
    """Return one ChangeSignatureRequest from one JSON value."""
    object_ = json_object(value)

    return ChangeSignatureRequest(
        position=destack._generated.query.protocol.target.from_json_position(
            json_field(object_, "position")
        ),
        new_parameters=json_string(json_field(object_, "newParameters")),
        new_arguments=json_string(json_field(object_, "newArguments")),
    )


@dataclass(frozen=True, slots=True)
class ChangeSignatureResponse:
    """Response payload for change signature queries."""

    # change signature edit, if available
    edit: destack._generated.source.edit.edit.PatchSet | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_change_signature_response(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ChangeSignatureResponse:
        """Decode one ChangeSignatureResponse."""
        return decode_change_signature_response(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_change_signature_response(self)

    @classmethod
    def from_json(cls, value: Json) -> ChangeSignatureResponse:
        """Return one ChangeSignatureResponse from one JSON value."""
        return from_json_change_signature_response(value)


def encode_change_signature_response(
    writer: BinaryWriter, value: ChangeSignatureResponse
) -> None:
    """Encode one ChangeSignatureResponse."""
    if value.edit is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.source.edit.edit.encode_patch_set(writer, value.edit)


def decode_change_signature_response(reader: BinaryReader) -> ChangeSignatureResponse:
    """Decode one ChangeSignatureResponse."""
    edit = reader.read_option(
        lambda: destack._generated.source.edit.edit.decode_patch_set(reader)
    )

    return ChangeSignatureResponse(
        edit=edit,
    )


def to_json_change_signature_response(value: ChangeSignatureResponse) -> Json:
    """Return one JSON value for one ChangeSignatureResponse."""
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


def from_json_change_signature_response(value: Json) -> ChangeSignatureResponse:
    """Return one ChangeSignatureResponse from one JSON value."""
    object_ = json_object(value)

    return ChangeSignatureResponse(
        edit=json_optional(
            object_,
            "edit",
            lambda value: destack._generated.source.edit.edit.from_json_patch_set(
                value
            ),
        ),
    )


__all__ = [
    "ChangeSignatureRequest",
    "encode_change_signature_request",
    "decode_change_signature_request",
    "to_json_change_signature_request",
    "from_json_change_signature_request",
    "ChangeSignatureResponse",
    "encode_change_signature_response",
    "decode_change_signature_response",
    "to_json_change_signature_response",
    "from_json_change_signature_response",
]
