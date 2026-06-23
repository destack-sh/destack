# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes

import destack._generated.protocol.notification
import destack._generated.protocol.workspace.file.image
import destack._generated.protocol.workspace.file.update
import destack._generated.protocol.workspace.message
import destack._generated.protocol.workspace.root

if TYPE_CHECKING:
    from destack._generated.protocol.notification import (
        DiagnosticBatch,
    )

    from destack._generated.protocol.workspace.file.image import (
        FileOperation,
    )

    from destack._generated.protocol.workspace.file.update import (
        Commit,
        SourceUpdate,
    )

    from destack._generated.protocol.workspace.message import (
        Message,
        UpdateBatch,
    )

    from destack._generated.protocol.workspace.root import (
        ReloadReason,
    )


@dataclass(frozen=True, slots=True)
class OpenRootRequest:
    """Request to open a root."""

    """The root path."""
    root: str
    """Root open options."""
    options: RootOpenOptions


def encode_open_root_request(writer: Writer, value: OpenRootRequest) -> None:
    writer.write_string(value.root)
    encode_root_open_options(writer, value.options)


def decode_open_root_request(reader: Reader) -> OpenRootRequest:
    field_0 = reader.read_string()
    field_1 = decode_root_open_options(reader)

    return OpenRootRequest(
        root=field_0,
        options=field_1,
    )


@dataclass(frozen=True, slots=True)
class RootOpenOptions:
    """Options for opening a root."""

    """Whether to preload root state."""
    load_index: bool


def encode_root_open_options(writer: Writer, value: RootOpenOptions) -> None:
    writer.write_bool(value.load_index)


def decode_root_open_options(reader: Reader) -> RootOpenOptions:
    field_0 = reader.read_bool()

    return RootOpenOptions(
        load_index=field_0,
    )


@dataclass(frozen=True, slots=True)
class CloseRootRequest:
    """Request to close a root handle."""

    """Handle to close."""
    handle: RootId


def encode_close_root_request(writer: Writer, value: CloseRootRequest) -> None:
    encode_root_id(writer, value.handle)


def decode_close_root_request(reader: Reader) -> CloseRootRequest:
    field_0 = decode_root_id(reader)

    return CloseRootRequest(
        handle=field_0,
    )


@dataclass(frozen=True, slots=True)
class RootId:
    """Unique identifier for an opened root."""

    field_0: int


def encode_root_id(writer: Writer, value: RootId) -> None:
    writer.write_unsigned(value.field_0)


def decode_root_id(reader: Reader) -> RootId:
    field_0 = reader.read_number()

    return RootId(
        field_0=field_0,
    )


@dataclass(frozen=True, slots=True)
class ReloadRootRequest:
    """Request to reload a root."""

    """Root handle."""
    handle: RootId
    """Reason for the reload."""
    reason: ReloadReason


def encode_reload_root_request(writer: Writer, value: ReloadRootRequest) -> None:
    encode_root_id(writer, value.handle)
    destack._generated.protocol.workspace.root.encode_reload_reason(
        writer, value.reason
    )


def decode_reload_root_request(reader: Reader) -> ReloadRootRequest:
    field_0 = decode_root_id(reader)
    field_1 = destack._generated.protocol.workspace.root.decode_reload_reason(reader)

    return ReloadRootRequest(
        handle=field_0,
        reason=field_1,
    )


@dataclass(frozen=True, slots=True)
class FileOperationRequest:
    """Request to apply a file operation."""

    """Root handle."""
    handle: RootId
    """Operation payload."""
    operation: FileOperation


def encode_file_operation_request(writer: Writer, value: FileOperationRequest) -> None:
    encode_root_id(writer, value.handle)
    destack._generated.protocol.workspace.file.image.encode_file_operation(
        writer, value.operation
    )


def decode_file_operation_request(reader: Reader) -> FileOperationRequest:
    field_0 = decode_root_id(reader)
    field_1 = destack._generated.protocol.workspace.file.image.decode_file_operation(
        reader
    )

    return FileOperationRequest(
        handle=field_0,
        operation=field_1,
    )


@dataclass(frozen=True, slots=True)
class SourceUpdateRequest:
    """Request to apply a source update."""

    """Root handle."""
    handle: RootId
    """Source update payload."""
    update: SourceUpdate


def encode_source_update_request(writer: Writer, value: SourceUpdateRequest) -> None:
    encode_root_id(writer, value.handle)
    destack._generated.protocol.workspace.file.update.encode_source_update(
        writer, value.update
    )


def decode_source_update_request(reader: Reader) -> SourceUpdateRequest:
    field_0 = decode_root_id(reader)
    field_1 = destack._generated.protocol.workspace.file.update.decode_source_update(
        reader
    )

    return SourceUpdateRequest(
        handle=field_0,
        update=field_1,
    )


@dataclass(frozen=True, slots=True)
class RootOpenedResponse:
    """Response to opening a root."""

    """Assigned root handle id."""
    handle: RootId
    """Canonical root path opened by the server."""
    root: str
    """Diagnostics produced during initialization."""
    diagnostics: Sequence[DiagnosticBatch]
    """Messages produced during initialization."""
    messages: Sequence[Message]


def encode_root_opened_response(writer: Writer, value: RootOpenedResponse) -> None:
    encode_root_id(writer, value.handle)
    writer.write_string(value.root)
    writer.write_unsigned(len(value.diagnostics))
    for item_0 in value.diagnostics:
        destack._generated.protocol.notification.encode_diagnostic_batch(writer, item_0)
    writer.write_unsigned(len(value.messages))
    for item_0 in value.messages:
        destack._generated.protocol.workspace.message.encode_message(writer, item_0)


def decode_root_opened_response(reader: Reader) -> RootOpenedResponse:
    field_0 = decode_root_id(reader)
    field_1 = reader.read_string()
    field_2 = [
        destack._generated.protocol.notification.decode_diagnostic_batch(reader)
        for _ in range(reader.read_number())
    ]
    field_3 = [
        destack._generated.protocol.workspace.message.decode_message(reader)
        for _ in range(reader.read_number())
    ]

    return RootOpenedResponse(
        handle=field_0,
        root=field_1,
        diagnostics=field_2,
        messages=field_3,
    )


@dataclass(frozen=True, slots=True)
class RootClosedResponse:
    """Response to closing a root."""

    """Closed handle id."""
    handle: RootId


def encode_root_closed_response(writer: Writer, value: RootClosedResponse) -> None:
    encode_root_id(writer, value.handle)


def decode_root_closed_response(reader: Reader) -> RootClosedResponse:
    field_0 = decode_root_id(reader)

    return RootClosedResponse(
        handle=field_0,
    )


@dataclass(frozen=True, slots=True)
class RootReloadResponse:
    """Response to root reloads."""

    """Root handle."""
    handle: RootId
    """Updates produced during reload."""
    updates: UpdateBatch


def encode_root_reload_response(writer: Writer, value: RootReloadResponse) -> None:
    encode_root_id(writer, value.handle)
    destack._generated.protocol.workspace.message.encode_update_batch(
        writer, value.updates
    )


def decode_root_reload_response(reader: Reader) -> RootReloadResponse:
    field_0 = decode_root_id(reader)
    field_1 = destack._generated.protocol.workspace.message.decode_update_batch(reader)

    return RootReloadResponse(
        handle=field_0,
        updates=field_1,
    )


@dataclass(frozen=True, slots=True)
class FileOperationResponse:
    """Response to a file operation."""

    """Root handle."""
    handle: RootId
    """Updates produced by the change."""
    updates: UpdateBatch


def encode_file_operation_response(
    writer: Writer, value: FileOperationResponse
) -> None:
    encode_root_id(writer, value.handle)
    destack._generated.protocol.workspace.message.encode_update_batch(
        writer, value.updates
    )


def decode_file_operation_response(reader: Reader) -> FileOperationResponse:
    field_0 = decode_root_id(reader)
    field_1 = destack._generated.protocol.workspace.message.decode_update_batch(reader)

    return FileOperationResponse(
        handle=field_0,
        updates=field_1,
    )


@dataclass(frozen=True, slots=True)
class SourceUpdateResponse:
    """Response to a source update."""

    """Root handle."""
    handle: RootId
    """Commit produced by the change."""
    commit: Commit


def encode_source_update_response(writer: Writer, value: SourceUpdateResponse) -> None:
    encode_root_id(writer, value.handle)
    destack._generated.protocol.workspace.file.update.encode_commit(
        writer, value.commit
    )


def decode_source_update_response(reader: Reader) -> SourceUpdateResponse:
    field_0 = decode_root_id(reader)
    field_1 = destack._generated.protocol.workspace.file.update.decode_commit(reader)

    return SourceUpdateResponse(
        handle=field_0,
        commit=field_1,
    )


__all__ = [
    "OpenRootRequest",
    "encode_open_root_request",
    "decode_open_root_request",
    "RootOpenOptions",
    "encode_root_open_options",
    "decode_root_open_options",
    "CloseRootRequest",
    "encode_close_root_request",
    "decode_close_root_request",
    "RootId",
    "encode_root_id",
    "decode_root_id",
    "ReloadRootRequest",
    "encode_reload_root_request",
    "decode_reload_root_request",
    "FileOperationRequest",
    "encode_file_operation_request",
    "decode_file_operation_request",
    "SourceUpdateRequest",
    "encode_source_update_request",
    "decode_source_update_request",
    "RootOpenedResponse",
    "encode_root_opened_response",
    "decode_root_opened_response",
    "RootClosedResponse",
    "encode_root_closed_response",
    "decode_root_closed_response",
    "RootReloadResponse",
    "encode_root_reload_response",
    "decode_root_reload_response",
    "FileOperationResponse",
    "encode_file_operation_response",
    "decode_file_operation_response",
    "SourceUpdateResponse",
    "encode_source_update_response",
    "decode_source_update_response",
]
