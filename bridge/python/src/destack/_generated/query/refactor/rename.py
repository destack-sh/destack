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
import destack._generated.source.file.model.span


@dataclass(frozen=True, slots=True)
class RenameTargetRequest:
    """Request the rename target at a cursor position."""

    # the queried position
    position: destack._generated.query.core.target.QueryPosition

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_rename_target_request(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> RenameTargetRequest:
        """Decode one RenameTargetRequest."""
        return decode_rename_target_request(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_rename_target_request(self)

    @classmethod
    def from_json(cls, value: Json) -> RenameTargetRequest:
        """Return one RenameTargetRequest from one JSON value."""
        return from_json_rename_target_request(value)


def encode_rename_target_request(
    writer: BinaryWriter, value: RenameTargetRequest
) -> None:
    """Encode one RenameTargetRequest."""
    destack._generated.query.core.target.encode_query_position(writer, value.position)


def decode_rename_target_request(reader: BinaryReader) -> RenameTargetRequest:
    """Decode one RenameTargetRequest."""
    position = destack._generated.query.core.target.decode_query_position(reader)

    return RenameTargetRequest(
        position=position,
    )


def to_json_rename_target_request(value: RenameTargetRequest) -> Json:
    """Return one JSON value for one RenameTargetRequest."""
    return {
        "position": destack._generated.query.core.target.to_json_query_position(
            value.position
        ),
    }


def from_json_rename_target_request(value: Json) -> RenameTargetRequest:
    """Return one RenameTargetRequest from one JSON value."""
    object_ = json_object(value)

    return RenameTargetRequest(
        position=destack._generated.query.core.target.from_json_query_position(
            json_field(object_, "position")
        ),
    )


@dataclass(frozen=True, slots=True)
class RenameRequest:
    """Request rename edits at a cursor position."""

    # the queried position
    position: destack._generated.query.core.target.QueryPosition
    # the new name for the symbol
    new_name: str

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_rename_request(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> RenameRequest:
        """Decode one RenameRequest."""
        return decode_rename_request(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_rename_request(self)

    @classmethod
    def from_json(cls, value: Json) -> RenameRequest:
        """Return one RenameRequest from one JSON value."""
        return from_json_rename_request(value)


def encode_rename_request(writer: BinaryWriter, value: RenameRequest) -> None:
    """Encode one RenameRequest."""
    destack._generated.query.core.target.encode_query_position(writer, value.position)
    writer.write_string(value.new_name)


def decode_rename_request(reader: BinaryReader) -> RenameRequest:
    """Decode one RenameRequest."""
    position = destack._generated.query.core.target.decode_query_position(reader)
    new_name = reader.read_string()

    return RenameRequest(
        position=position,
        new_name=new_name,
    )


def to_json_rename_request(value: RenameRequest) -> Json:
    """Return one JSON value for one RenameRequest."""
    return {
        "position": destack._generated.query.core.target.to_json_query_position(
            value.position
        ),
        "newName": value.new_name,
    }


def from_json_rename_request(value: Json) -> RenameRequest:
    """Return one RenameRequest from one JSON value."""
    object_ = json_object(value)

    return RenameRequest(
        position=destack._generated.query.core.target.from_json_query_position(
            json_field(object_, "position")
        ),
        new_name=json_string(json_field(object_, "newName")),
    )


@dataclass(frozen=True, slots=True)
class RenameTargetResponse:
    """Response payload for rename target queries."""

    # rename target, if available
    result: RenameTarget | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_rename_target_response(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> RenameTargetResponse:
        """Decode one RenameTargetResponse."""
        return decode_rename_target_response(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_rename_target_response(self)

    @classmethod
    def from_json(cls, value: Json) -> RenameTargetResponse:
        """Return one RenameTargetResponse from one JSON value."""
        return from_json_rename_target_response(value)


def encode_rename_target_response(
    writer: BinaryWriter, value: RenameTargetResponse
) -> None:
    """Encode one RenameTargetResponse."""
    if value.result is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_rename_target(writer, value.result)


def decode_rename_target_response(reader: BinaryReader) -> RenameTargetResponse:
    """Decode one RenameTargetResponse."""
    result = reader.read_option(lambda: decode_rename_target(reader))

    return RenameTargetResponse(
        result=result,
    )


def to_json_rename_target_response(value: RenameTargetResponse) -> Json:
    """Return one JSON value for one RenameTargetResponse."""
    return {
        **(
            {}
            if value.result is None
            else {"result": to_json_rename_target(value.result)}
        ),
    }


def from_json_rename_target_response(value: Json) -> RenameTargetResponse:
    """Return one RenameTargetResponse from one JSON value."""
    object_ = json_object(value)

    return RenameTargetResponse(
        result=json_optional(
            object_, "result", lambda value: from_json_rename_target(value)
        ),
    )


@dataclass(frozen=True, slots=True)
class RenameTarget:
    """Target of a rename query."""

    # the semantic rename target
    target: destack._generated.query.core.target.QueryTarget
    # the range of the symbol to rename
    range: destack._generated.source.file.model.span.Span
    # the current name (placeholder for rename dialog)
    placeholder: str

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_rename_target(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> RenameTarget:
        """Decode one RenameTarget."""
        return decode_rename_target(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_rename_target(self)

    @classmethod
    def from_json(cls, value: Json) -> RenameTarget:
        """Return one RenameTarget from one JSON value."""
        return from_json_rename_target(value)


def encode_rename_target(writer: BinaryWriter, value: RenameTarget) -> None:
    """Encode one RenameTarget."""
    destack._generated.query.core.target.encode_query_target(writer, value.target)
    destack._generated.source.file.model.span.encode_span(writer, value.range)
    writer.write_string(value.placeholder)


def decode_rename_target(reader: BinaryReader) -> RenameTarget:
    """Decode one RenameTarget."""
    target = destack._generated.query.core.target.decode_query_target(reader)
    range_ = destack._generated.source.file.model.span.decode_span(reader)
    placeholder = reader.read_string()

    return RenameTarget(
        target=target,
        range=range_,
        placeholder=placeholder,
    )


def to_json_rename_target(value: RenameTarget) -> Json:
    """Return one JSON value for one RenameTarget."""
    return {
        "target": destack._generated.query.core.target.to_json_query_target(
            value.target
        ),
        "range": destack._generated.source.file.model.span.to_json_span(value.range),
        "placeholder": value.placeholder,
    }


def from_json_rename_target(value: Json) -> RenameTarget:
    """Return one RenameTarget from one JSON value."""
    object_ = json_object(value)

    return RenameTarget(
        target=destack._generated.query.core.target.from_json_query_target(
            json_field(object_, "target")
        ),
        range=destack._generated.source.file.model.span.from_json_span(
            json_field(object_, "range")
        ),
        placeholder=json_string(json_field(object_, "placeholder")),
    )


@dataclass(frozen=True, slots=True)
class RenameResponse:
    """Response payload for rename queries."""

    # rename edit, if available
    edit: destack._generated.source.edit.edit.PatchSet | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_rename_response(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> RenameResponse:
        """Decode one RenameResponse."""
        return decode_rename_response(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_rename_response(self)

    @classmethod
    def from_json(cls, value: Json) -> RenameResponse:
        """Return one RenameResponse from one JSON value."""
        return from_json_rename_response(value)


def encode_rename_response(writer: BinaryWriter, value: RenameResponse) -> None:
    """Encode one RenameResponse."""
    if value.edit is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.source.edit.edit.encode_patch_set(writer, value.edit)


def decode_rename_response(reader: BinaryReader) -> RenameResponse:
    """Decode one RenameResponse."""
    edit = reader.read_option(
        lambda: destack._generated.source.edit.edit.decode_patch_set(reader)
    )

    return RenameResponse(
        edit=edit,
    )


def to_json_rename_response(value: RenameResponse) -> Json:
    """Return one JSON value for one RenameResponse."""
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


def from_json_rename_response(value: Json) -> RenameResponse:
    """Return one RenameResponse from one JSON value."""
    object_ = json_object(value)

    return RenameResponse(
        edit=json_optional(
            object_,
            "edit",
            lambda value: destack._generated.source.edit.edit.from_json_patch_set(
                value
            ),
        ),
    )


__all__ = [
    "RenameTargetRequest",
    "encode_rename_target_request",
    "decode_rename_target_request",
    "to_json_rename_target_request",
    "from_json_rename_target_request",
    "RenameRequest",
    "encode_rename_request",
    "decode_rename_request",
    "to_json_rename_request",
    "from_json_rename_request",
    "RenameTargetResponse",
    "encode_rename_target_response",
    "decode_rename_target_response",
    "to_json_rename_target_response",
    "from_json_rename_target_response",
    "RenameTarget",
    "encode_rename_target",
    "decode_rename_target",
    "to_json_rename_target",
    "from_json_rename_target",
    "RenameResponse",
    "encode_rename_response",
    "decode_rename_response",
    "to_json_rename_response",
    "from_json_rename_response",
]
