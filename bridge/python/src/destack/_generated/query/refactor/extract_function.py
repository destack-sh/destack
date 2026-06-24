# generated bridge target, do not edit

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
class ExtractFunctionRequest:
    """Request payload for extract function queries."""

    # the selected source range
    range: destack._generated.query.core.target.QueryRange
    # the name for the extracted function
    new_name: str

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_extract_function_request(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ExtractFunctionRequest:
        """Decode one ExtractFunctionRequest."""
        return decode_extract_function_request(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_extract_function_request(self)

    @classmethod
    def from_json(cls, value: Json) -> ExtractFunctionRequest:
        """Return one ExtractFunctionRequest from one JSON value."""
        return from_json_extract_function_request(value)


def encode_extract_function_request(
    writer: BinaryWriter, value: ExtractFunctionRequest
) -> None:
    """Encode one ExtractFunctionRequest."""
    destack._generated.query.core.target.encode_query_range(writer, value.range)
    writer.write_string(value.new_name)


def decode_extract_function_request(reader: BinaryReader) -> ExtractFunctionRequest:
    """Decode one ExtractFunctionRequest."""
    range_ = destack._generated.query.core.target.decode_query_range(reader)
    new_name = reader.read_string()

    return ExtractFunctionRequest(
        range=range_,
        new_name=new_name,
    )


def to_json_extract_function_request(value: ExtractFunctionRequest) -> Json:
    """Return one JSON value for one ExtractFunctionRequest."""
    return {
        "range": destack._generated.query.core.target.to_json_query_range(value.range),
        "newName": value.new_name,
    }


def from_json_extract_function_request(value: Json) -> ExtractFunctionRequest:
    """Return one ExtractFunctionRequest from one JSON value."""
    object_ = json_object(value)

    return ExtractFunctionRequest(
        range=destack._generated.query.core.target.from_json_query_range(
            json_field(object_, "range")
        ),
        new_name=json_string(json_field(object_, "newName")),
    )


@dataclass(frozen=True, slots=True)
class ExtractFunctionResponse:
    """Response payload for extract function queries."""

    # extract function edit, if available
    edit: destack._generated.source.edit.edit.PatchSet | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_extract_function_response(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ExtractFunctionResponse:
        """Decode one ExtractFunctionResponse."""
        return decode_extract_function_response(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_extract_function_response(self)

    @classmethod
    def from_json(cls, value: Json) -> ExtractFunctionResponse:
        """Return one ExtractFunctionResponse from one JSON value."""
        return from_json_extract_function_response(value)


def encode_extract_function_response(
    writer: BinaryWriter, value: ExtractFunctionResponse
) -> None:
    """Encode one ExtractFunctionResponse."""
    if value.edit is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.source.edit.edit.encode_patch_set(writer, value.edit)


def decode_extract_function_response(reader: BinaryReader) -> ExtractFunctionResponse:
    """Decode one ExtractFunctionResponse."""
    edit = reader.read_option(
        lambda: destack._generated.source.edit.edit.decode_patch_set(reader)
    )

    return ExtractFunctionResponse(
        edit=edit,
    )


def to_json_extract_function_response(value: ExtractFunctionResponse) -> Json:
    """Return one JSON value for one ExtractFunctionResponse."""
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


def from_json_extract_function_response(value: Json) -> ExtractFunctionResponse:
    """Return one ExtractFunctionResponse from one JSON value."""
    object_ = json_object(value)

    return ExtractFunctionResponse(
        edit=json_optional(
            object_,
            "edit",
            lambda value: destack._generated.source.edit.edit.from_json_patch_set(
                value
            ),
        ),
    )


__all__ = [
    "ExtractFunctionRequest",
    "encode_extract_function_request",
    "decode_extract_function_request",
    "to_json_extract_function_request",
    "from_json_extract_function_request",
    "ExtractFunctionResponse",
    "encode_extract_function_response",
    "decode_extract_function_response",
    "to_json_extract_function_response",
    "from_json_extract_function_response",
]
