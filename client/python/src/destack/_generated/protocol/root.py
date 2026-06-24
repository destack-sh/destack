# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_array,
    json_bool,
    json_field,
    json_int,
    json_object,
    json_string,
)

import destack._generated.protocol.notification
import destack._generated.protocol.workspace.file.image
import destack._generated.protocol.workspace.file.update
import destack._generated.protocol.workspace.message
import destack._generated.protocol.workspace.root


@dataclass(frozen=True, slots=True)
class OpenRootRequest:
    """Request to open a root."""

    # the root path
    root: str
    # root open options
    options: RootOpenOptions

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_open_root_request(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> OpenRootRequest:
        """Decode one OpenRootRequest."""
        return decode_open_root_request(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_open_root_request(self)

    @classmethod
    def from_json(cls, value: Json) -> OpenRootRequest:
        """Return one OpenRootRequest from one JSON value."""
        return from_json_open_root_request(value)


def encode_open_root_request(writer: BinaryWriter, value: OpenRootRequest) -> None:
    """Encode one OpenRootRequest."""
    writer.write_string(value.root)
    encode_root_open_options(writer, value.options)


def decode_open_root_request(reader: BinaryReader) -> OpenRootRequest:
    """Decode one OpenRootRequest."""
    root = reader.read_string()
    options = decode_root_open_options(reader)

    return OpenRootRequest(
        root=root,
        options=options,
    )


def to_json_open_root_request(value: OpenRootRequest) -> Json:
    """Return one JSON value for one OpenRootRequest."""
    return {
        "root": value.root,
        "options": to_json_root_open_options(value.options),
    }


def from_json_open_root_request(value: Json) -> OpenRootRequest:
    """Return one OpenRootRequest from one JSON value."""
    object_ = json_object(value)

    return OpenRootRequest(
        root=json_string(json_field(object_, "root")),
        options=from_json_root_open_options(json_field(object_, "options")),
    )


@dataclass(frozen=True, slots=True)
class RootOpenOptions:
    """Options for opening a root."""

    # whether to preload root state
    load_index: bool

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_root_open_options(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> RootOpenOptions:
        """Decode one RootOpenOptions."""
        return decode_root_open_options(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_root_open_options(self)

    @classmethod
    def from_json(cls, value: Json) -> RootOpenOptions:
        """Return one RootOpenOptions from one JSON value."""
        return from_json_root_open_options(value)


def encode_root_open_options(writer: BinaryWriter, value: RootOpenOptions) -> None:
    """Encode one RootOpenOptions."""
    writer.write_bool(value.load_index)


def decode_root_open_options(reader: BinaryReader) -> RootOpenOptions:
    """Decode one RootOpenOptions."""
    load_index = reader.read_bool()

    return RootOpenOptions(
        load_index=load_index,
    )


def to_json_root_open_options(value: RootOpenOptions) -> Json:
    """Return one JSON value for one RootOpenOptions."""
    return {
        "loadIndex": value.load_index,
    }


def from_json_root_open_options(value: Json) -> RootOpenOptions:
    """Return one RootOpenOptions from one JSON value."""
    object_ = json_object(value)

    return RootOpenOptions(
        load_index=json_bool(json_field(object_, "loadIndex")),
    )


@dataclass(frozen=True, slots=True)
class CloseRootRequest:
    """Request to close a root handle."""

    # handle to close
    handle: RootId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_close_root_request(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> CloseRootRequest:
        """Decode one CloseRootRequest."""
        return decode_close_root_request(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_close_root_request(self)

    @classmethod
    def from_json(cls, value: Json) -> CloseRootRequest:
        """Return one CloseRootRequest from one JSON value."""
        return from_json_close_root_request(value)


def encode_close_root_request(writer: BinaryWriter, value: CloseRootRequest) -> None:
    """Encode one CloseRootRequest."""
    encode_root_id(writer, value.handle)


def decode_close_root_request(reader: BinaryReader) -> CloseRootRequest:
    """Decode one CloseRootRequest."""
    handle = decode_root_id(reader)

    return CloseRootRequest(
        handle=handle,
    )


def to_json_close_root_request(value: CloseRootRequest) -> Json:
    """Return one JSON value for one CloseRootRequest."""
    return {
        "handle": to_json_root_id(value.handle),
    }


def from_json_close_root_request(value: Json) -> CloseRootRequest:
    """Return one CloseRootRequest from one JSON value."""
    object_ = json_object(value)

    return CloseRootRequest(
        handle=from_json_root_id(json_field(object_, "handle")),
    )


"""Unique identifier for an opened root."""
RootId: typing.TypeAlias = int


def encode_root_id(writer: BinaryWriter, value: RootId) -> None:
    """Encode one RootId."""
    writer.write_unsigned(value)


def decode_root_id(reader: BinaryReader) -> RootId:
    """Decode one RootId."""
    return reader.read_number()


def to_json_root_id(value: RootId) -> Json:
    """Return one JSON value for one RootId."""
    return value


def from_json_root_id(value: Json) -> RootId:
    """Return one RootId from one JSON value."""
    return json_int(value)


@dataclass(frozen=True, slots=True)
class ReloadRootRequest:
    """Request to reload a root."""

    # root handle
    handle: RootId
    # reason for the reload
    reason: destack._generated.protocol.workspace.root.ReloadReason

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_reload_root_request(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ReloadRootRequest:
        """Decode one ReloadRootRequest."""
        return decode_reload_root_request(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_reload_root_request(self)

    @classmethod
    def from_json(cls, value: Json) -> ReloadRootRequest:
        """Return one ReloadRootRequest from one JSON value."""
        return from_json_reload_root_request(value)


def encode_reload_root_request(writer: BinaryWriter, value: ReloadRootRequest) -> None:
    """Encode one ReloadRootRequest."""
    encode_root_id(writer, value.handle)
    destack._generated.protocol.workspace.root.encode_reload_reason(
        writer, value.reason
    )


def decode_reload_root_request(reader: BinaryReader) -> ReloadRootRequest:
    """Decode one ReloadRootRequest."""
    handle = decode_root_id(reader)
    reason = destack._generated.protocol.workspace.root.decode_reload_reason(reader)

    return ReloadRootRequest(
        handle=handle,
        reason=reason,
    )


def to_json_reload_root_request(value: ReloadRootRequest) -> Json:
    """Return one JSON value for one ReloadRootRequest."""
    return {
        "handle": to_json_root_id(value.handle),
        "reason": destack._generated.protocol.workspace.root.to_json_reload_reason(
            value.reason
        ),
    }


def from_json_reload_root_request(value: Json) -> ReloadRootRequest:
    """Return one ReloadRootRequest from one JSON value."""
    object_ = json_object(value)

    return ReloadRootRequest(
        handle=from_json_root_id(json_field(object_, "handle")),
        reason=destack._generated.protocol.workspace.root.from_json_reload_reason(
            json_field(object_, "reason")
        ),
    )


@dataclass(frozen=True, slots=True)
class FileOperationRequest:
    """Request to apply a file operation."""

    # root handle
    handle: RootId
    # operation payload
    operation: destack._generated.protocol.workspace.file.image.FileOperation

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_file_operation_request(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> FileOperationRequest:
        """Decode one FileOperationRequest."""
        return decode_file_operation_request(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_file_operation_request(self)

    @classmethod
    def from_json(cls, value: Json) -> FileOperationRequest:
        """Return one FileOperationRequest from one JSON value."""
        return from_json_file_operation_request(value)


def encode_file_operation_request(
    writer: BinaryWriter, value: FileOperationRequest
) -> None:
    """Encode one FileOperationRequest."""
    encode_root_id(writer, value.handle)
    destack._generated.protocol.workspace.file.image.encode_file_operation(
        writer, value.operation
    )


def decode_file_operation_request(reader: BinaryReader) -> FileOperationRequest:
    """Decode one FileOperationRequest."""
    handle = decode_root_id(reader)
    operation = destack._generated.protocol.workspace.file.image.decode_file_operation(
        reader
    )

    return FileOperationRequest(
        handle=handle,
        operation=operation,
    )


def to_json_file_operation_request(value: FileOperationRequest) -> Json:
    """Return one JSON value for one FileOperationRequest."""
    return {
        "handle": to_json_root_id(value.handle),
        "operation": destack._generated.protocol.workspace.file.image.to_json_file_operation(
            value.operation
        ),
    }


def from_json_file_operation_request(value: Json) -> FileOperationRequest:
    """Return one FileOperationRequest from one JSON value."""
    object_ = json_object(value)

    return FileOperationRequest(
        handle=from_json_root_id(json_field(object_, "handle")),
        operation=destack._generated.protocol.workspace.file.image.from_json_file_operation(
            json_field(object_, "operation")
        ),
    )


@dataclass(frozen=True, slots=True)
class SourceUpdateRequest:
    """Request to apply a source update."""

    # root handle
    handle: RootId
    # source update payload
    update: destack._generated.protocol.workspace.file.update.SourceUpdate

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_source_update_request(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> SourceUpdateRequest:
        """Decode one SourceUpdateRequest."""
        return decode_source_update_request(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_source_update_request(self)

    @classmethod
    def from_json(cls, value: Json) -> SourceUpdateRequest:
        """Return one SourceUpdateRequest from one JSON value."""
        return from_json_source_update_request(value)


def encode_source_update_request(
    writer: BinaryWriter, value: SourceUpdateRequest
) -> None:
    """Encode one SourceUpdateRequest."""
    encode_root_id(writer, value.handle)
    destack._generated.protocol.workspace.file.update.encode_source_update(
        writer, value.update
    )


def decode_source_update_request(reader: BinaryReader) -> SourceUpdateRequest:
    """Decode one SourceUpdateRequest."""
    handle = decode_root_id(reader)
    update = destack._generated.protocol.workspace.file.update.decode_source_update(
        reader
    )

    return SourceUpdateRequest(
        handle=handle,
        update=update,
    )


def to_json_source_update_request(value: SourceUpdateRequest) -> Json:
    """Return one JSON value for one SourceUpdateRequest."""
    return {
        "handle": to_json_root_id(value.handle),
        "update": destack._generated.protocol.workspace.file.update.to_json_source_update(
            value.update
        ),
    }


def from_json_source_update_request(value: Json) -> SourceUpdateRequest:
    """Return one SourceUpdateRequest from one JSON value."""
    object_ = json_object(value)

    return SourceUpdateRequest(
        handle=from_json_root_id(json_field(object_, "handle")),
        update=destack._generated.protocol.workspace.file.update.from_json_source_update(
            json_field(object_, "update")
        ),
    )


@dataclass(frozen=True, slots=True)
class RootOpenedResponse:
    """Response to opening a root."""

    # assigned root handle id
    handle: RootId
    # canonical root path opened by the server
    root: str
    # diagnostics produced during initialization
    diagnostics: Sequence[destack._generated.protocol.notification.DiagnosticBatch]
    # messages produced during initialization
    messages: Sequence[destack._generated.protocol.workspace.message.Message]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_root_opened_response(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> RootOpenedResponse:
        """Decode one RootOpenedResponse."""
        return decode_root_opened_response(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_root_opened_response(self)

    @classmethod
    def from_json(cls, value: Json) -> RootOpenedResponse:
        """Return one RootOpenedResponse from one JSON value."""
        return from_json_root_opened_response(value)


def encode_root_opened_response(
    writer: BinaryWriter, value: RootOpenedResponse
) -> None:
    """Encode one RootOpenedResponse."""
    encode_root_id(writer, value.handle)
    writer.write_string(value.root)
    writer.write_unsigned(len(value.diagnostics))
    for item_value_diagnostics_0 in value.diagnostics:
        destack._generated.protocol.notification.encode_diagnostic_batch(
            writer, item_value_diagnostics_0
        )
    writer.write_unsigned(len(value.messages))
    for item_value_messages_0 in value.messages:
        destack._generated.protocol.workspace.message.encode_message(
            writer, item_value_messages_0
        )


def decode_root_opened_response(reader: BinaryReader) -> RootOpenedResponse:
    """Decode one RootOpenedResponse."""
    handle = decode_root_id(reader)
    root = reader.read_string()
    diagnostics = [
        destack._generated.protocol.notification.decode_diagnostic_batch(reader)
        for _ in range(reader.read_number())
    ]
    messages = [
        destack._generated.protocol.workspace.message.decode_message(reader)
        for _ in range(reader.read_number())
    ]

    return RootOpenedResponse(
        handle=handle,
        root=root,
        diagnostics=diagnostics,
        messages=messages,
    )


def to_json_root_opened_response(value: RootOpenedResponse) -> Json:
    """Return one JSON value for one RootOpenedResponse."""
    return {
        "handle": to_json_root_id(value.handle),
        "root": value.root,
        "diagnostics": [
            destack._generated.protocol.notification.to_json_diagnostic_batch(item_0)
            for item_0 in value.diagnostics
        ],
        "messages": [
            destack._generated.protocol.workspace.message.to_json_message(item_0)
            for item_0 in value.messages
        ],
    }


def from_json_root_opened_response(value: Json) -> RootOpenedResponse:
    """Return one RootOpenedResponse from one JSON value."""
    object_ = json_object(value)

    return RootOpenedResponse(
        handle=from_json_root_id(json_field(object_, "handle")),
        root=json_string(json_field(object_, "root")),
        diagnostics=[
            destack._generated.protocol.notification.from_json_diagnostic_batch(item_0)
            for item_0 in json_array(json_field(object_, "diagnostics"))
        ],
        messages=[
            destack._generated.protocol.workspace.message.from_json_message(item_0)
            for item_0 in json_array(json_field(object_, "messages"))
        ],
    )


@dataclass(frozen=True, slots=True)
class RootClosedResponse:
    """Response to closing a root."""

    # closed handle id
    handle: RootId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_root_closed_response(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> RootClosedResponse:
        """Decode one RootClosedResponse."""
        return decode_root_closed_response(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_root_closed_response(self)

    @classmethod
    def from_json(cls, value: Json) -> RootClosedResponse:
        """Return one RootClosedResponse from one JSON value."""
        return from_json_root_closed_response(value)


def encode_root_closed_response(
    writer: BinaryWriter, value: RootClosedResponse
) -> None:
    """Encode one RootClosedResponse."""
    encode_root_id(writer, value.handle)


def decode_root_closed_response(reader: BinaryReader) -> RootClosedResponse:
    """Decode one RootClosedResponse."""
    handle = decode_root_id(reader)

    return RootClosedResponse(
        handle=handle,
    )


def to_json_root_closed_response(value: RootClosedResponse) -> Json:
    """Return one JSON value for one RootClosedResponse."""
    return {
        "handle": to_json_root_id(value.handle),
    }


def from_json_root_closed_response(value: Json) -> RootClosedResponse:
    """Return one RootClosedResponse from one JSON value."""
    object_ = json_object(value)

    return RootClosedResponse(
        handle=from_json_root_id(json_field(object_, "handle")),
    )


@dataclass(frozen=True, slots=True)
class RootReloadResponse:
    """Response to root reloads."""

    # root handle
    handle: RootId
    # updates produced during reload
    updates: destack._generated.protocol.workspace.message.UpdateBatch

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_root_reload_response(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> RootReloadResponse:
        """Decode one RootReloadResponse."""
        return decode_root_reload_response(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_root_reload_response(self)

    @classmethod
    def from_json(cls, value: Json) -> RootReloadResponse:
        """Return one RootReloadResponse from one JSON value."""
        return from_json_root_reload_response(value)


def encode_root_reload_response(
    writer: BinaryWriter, value: RootReloadResponse
) -> None:
    """Encode one RootReloadResponse."""
    encode_root_id(writer, value.handle)
    destack._generated.protocol.workspace.message.encode_update_batch(
        writer, value.updates
    )


def decode_root_reload_response(reader: BinaryReader) -> RootReloadResponse:
    """Decode one RootReloadResponse."""
    handle = decode_root_id(reader)
    updates = destack._generated.protocol.workspace.message.decode_update_batch(reader)

    return RootReloadResponse(
        handle=handle,
        updates=updates,
    )


def to_json_root_reload_response(value: RootReloadResponse) -> Json:
    """Return one JSON value for one RootReloadResponse."""
    return {
        "handle": to_json_root_id(value.handle),
        "updates": destack._generated.protocol.workspace.message.to_json_update_batch(
            value.updates
        ),
    }


def from_json_root_reload_response(value: Json) -> RootReloadResponse:
    """Return one RootReloadResponse from one JSON value."""
    object_ = json_object(value)

    return RootReloadResponse(
        handle=from_json_root_id(json_field(object_, "handle")),
        updates=destack._generated.protocol.workspace.message.from_json_update_batch(
            json_field(object_, "updates")
        ),
    )


@dataclass(frozen=True, slots=True)
class FileOperationResponse:
    """Response to a file operation."""

    # root handle
    handle: RootId
    # updates produced by the change
    updates: destack._generated.protocol.workspace.message.UpdateBatch

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_file_operation_response(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> FileOperationResponse:
        """Decode one FileOperationResponse."""
        return decode_file_operation_response(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_file_operation_response(self)

    @classmethod
    def from_json(cls, value: Json) -> FileOperationResponse:
        """Return one FileOperationResponse from one JSON value."""
        return from_json_file_operation_response(value)


def encode_file_operation_response(
    writer: BinaryWriter, value: FileOperationResponse
) -> None:
    """Encode one FileOperationResponse."""
    encode_root_id(writer, value.handle)
    destack._generated.protocol.workspace.message.encode_update_batch(
        writer, value.updates
    )


def decode_file_operation_response(reader: BinaryReader) -> FileOperationResponse:
    """Decode one FileOperationResponse."""
    handle = decode_root_id(reader)
    updates = destack._generated.protocol.workspace.message.decode_update_batch(reader)

    return FileOperationResponse(
        handle=handle,
        updates=updates,
    )


def to_json_file_operation_response(value: FileOperationResponse) -> Json:
    """Return one JSON value for one FileOperationResponse."""
    return {
        "handle": to_json_root_id(value.handle),
        "updates": destack._generated.protocol.workspace.message.to_json_update_batch(
            value.updates
        ),
    }


def from_json_file_operation_response(value: Json) -> FileOperationResponse:
    """Return one FileOperationResponse from one JSON value."""
    object_ = json_object(value)

    return FileOperationResponse(
        handle=from_json_root_id(json_field(object_, "handle")),
        updates=destack._generated.protocol.workspace.message.from_json_update_batch(
            json_field(object_, "updates")
        ),
    )


@dataclass(frozen=True, slots=True)
class SourceUpdateResponse:
    """Response to a source update."""

    # root handle
    handle: RootId
    # commit produced by the change
    commit: destack._generated.protocol.workspace.file.update.Commit

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_source_update_response(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> SourceUpdateResponse:
        """Decode one SourceUpdateResponse."""
        return decode_source_update_response(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_source_update_response(self)

    @classmethod
    def from_json(cls, value: Json) -> SourceUpdateResponse:
        """Return one SourceUpdateResponse from one JSON value."""
        return from_json_source_update_response(value)


def encode_source_update_response(
    writer: BinaryWriter, value: SourceUpdateResponse
) -> None:
    """Encode one SourceUpdateResponse."""
    encode_root_id(writer, value.handle)
    destack._generated.protocol.workspace.file.update.encode_commit(
        writer, value.commit
    )


def decode_source_update_response(reader: BinaryReader) -> SourceUpdateResponse:
    """Decode one SourceUpdateResponse."""
    handle = decode_root_id(reader)
    commit = destack._generated.protocol.workspace.file.update.decode_commit(reader)

    return SourceUpdateResponse(
        handle=handle,
        commit=commit,
    )


def to_json_source_update_response(value: SourceUpdateResponse) -> Json:
    """Return one JSON value for one SourceUpdateResponse."""
    return {
        "handle": to_json_root_id(value.handle),
        "commit": destack._generated.protocol.workspace.file.update.to_json_commit(
            value.commit
        ),
    }


def from_json_source_update_response(value: Json) -> SourceUpdateResponse:
    """Return one SourceUpdateResponse from one JSON value."""
    object_ = json_object(value)

    return SourceUpdateResponse(
        handle=from_json_root_id(json_field(object_, "handle")),
        commit=destack._generated.protocol.workspace.file.update.from_json_commit(
            json_field(object_, "commit")
        ),
    )


__all__ = [
    "OpenRootRequest",
    "encode_open_root_request",
    "decode_open_root_request",
    "to_json_open_root_request",
    "from_json_open_root_request",
    "RootOpenOptions",
    "encode_root_open_options",
    "decode_root_open_options",
    "to_json_root_open_options",
    "from_json_root_open_options",
    "CloseRootRequest",
    "encode_close_root_request",
    "decode_close_root_request",
    "to_json_close_root_request",
    "from_json_close_root_request",
    "RootId",
    "encode_root_id",
    "decode_root_id",
    "to_json_root_id",
    "from_json_root_id",
    "ReloadRootRequest",
    "encode_reload_root_request",
    "decode_reload_root_request",
    "to_json_reload_root_request",
    "from_json_reload_root_request",
    "FileOperationRequest",
    "encode_file_operation_request",
    "decode_file_operation_request",
    "to_json_file_operation_request",
    "from_json_file_operation_request",
    "SourceUpdateRequest",
    "encode_source_update_request",
    "decode_source_update_request",
    "to_json_source_update_request",
    "from_json_source_update_request",
    "RootOpenedResponse",
    "encode_root_opened_response",
    "decode_root_opened_response",
    "to_json_root_opened_response",
    "from_json_root_opened_response",
    "RootClosedResponse",
    "encode_root_closed_response",
    "decode_root_closed_response",
    "to_json_root_closed_response",
    "from_json_root_closed_response",
    "RootReloadResponse",
    "encode_root_reload_response",
    "decode_root_reload_response",
    "to_json_root_reload_response",
    "from_json_root_reload_response",
    "FileOperationResponse",
    "encode_file_operation_response",
    "decode_file_operation_response",
    "to_json_file_operation_response",
    "from_json_file_operation_response",
    "SourceUpdateResponse",
    "encode_source_update_response",
    "decode_source_update_response",
    "to_json_source_update_response",
    "from_json_source_update_response",
]
