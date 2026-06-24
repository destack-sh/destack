# generated bridge target, do not edit

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

import destack._generated.protocol.payload
import destack._generated.protocol.root
import destack._generated.protocol.workspace.command.common
import destack._generated.protocol.workspace.message
import destack._generated.source.diagnostic.diagnostic
import destack._generated.source.file.model.file


@dataclass(frozen=True, slots=True)
class DiagnosticBatch:
    """Diagnostic batch for notifications."""

    # the file id for these diagnostics
    file_id: destack._generated.source.file.model.file.FileId
    # diagnostics for the file
    diagnostics: Sequence[destack._generated.source.diagnostic.diagnostic.Diagnostic]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_diagnostic_batch(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> DiagnosticBatch:
        """Decode one DiagnosticBatch."""
        return decode_diagnostic_batch(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_diagnostic_batch(self)

    @classmethod
    def from_json(cls, value: Json) -> DiagnosticBatch:
        """Return one DiagnosticBatch from one JSON value."""
        return from_json_diagnostic_batch(value)


def encode_diagnostic_batch(writer: BinaryWriter, value: DiagnosticBatch) -> None:
    """Encode one DiagnosticBatch."""
    destack._generated.source.file.model.file.encode_file_id(writer, value.file_id)
    writer.write_unsigned(len(value.diagnostics))
    for item_value_diagnostics_0 in value.diagnostics:
        destack._generated.source.diagnostic.diagnostic.encode_diagnostic(
            writer, item_value_diagnostics_0
        )


def decode_diagnostic_batch(reader: BinaryReader) -> DiagnosticBatch:
    """Decode one DiagnosticBatch."""
    file_id = destack._generated.source.file.model.file.decode_file_id(reader)
    diagnostics = [
        destack._generated.source.diagnostic.diagnostic.decode_diagnostic(reader)
        for _ in range(reader.read_number())
    ]

    return DiagnosticBatch(
        file_id=file_id,
        diagnostics=diagnostics,
    )


def to_json_diagnostic_batch(value: DiagnosticBatch) -> Json:
    """Return one JSON value for one DiagnosticBatch."""
    return {
        "fileId": destack._generated.source.file.model.file.to_json_file_id(
            value.file_id
        ),
        "diagnostics": [
            destack._generated.source.diagnostic.diagnostic.to_json_diagnostic(item_0)
            for item_0 in value.diagnostics
        ],
    }


def from_json_diagnostic_batch(value: Json) -> DiagnosticBatch:
    """Return one DiagnosticBatch from one JSON value."""
    object_ = json_object(value)

    return DiagnosticBatch(
        file_id=destack._generated.source.file.model.file.from_json_file_id(
            json_field(object_, "fileId")
        ),
        diagnostics=[
            destack._generated.source.diagnostic.diagnostic.from_json_diagnostic(item_0)
            for item_0 in json_array(json_field(object_, "diagnostics"))
        ],
    )


@dataclass(frozen=True, slots=True)
class WorkspaceNotificationDiagnostics:
    """Publish diagnostics for a root."""

    diagnostics: DiagnosticsNotification
    kind: typing.Literal["diagnostics"] = "diagnostics"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_notification(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_notification(self)


@dataclass(frozen=True, slots=True)
class WorkspaceNotificationMessages:
    """Publish workspace messages."""

    messages: MessageNotification
    kind: typing.Literal["messages"] = "messages"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_notification(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_notification(self)


@dataclass(frozen=True, slots=True)
class WorkspaceNotificationProgress:
    """Publish progress updates."""

    progress: ProgressNotification
    kind: typing.Literal["progress"] = "progress"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_notification(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_notification(self)


@dataclass(frozen=True, slots=True)
class WorkspaceNotificationPayloadChunk:
    """Publish chunked payload data."""

    payload_chunk: destack._generated.protocol.payload.PayloadChunkNotification
    kind: typing.Literal["payloadChunk"] = "payloadChunk"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_notification(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_notification(self)


"""Notifications emitted by the workspace protocol."""
WorkspaceNotification: typing.TypeAlias = (
    WorkspaceNotificationDiagnostics
    | WorkspaceNotificationMessages
    | WorkspaceNotificationProgress
    | WorkspaceNotificationPayloadChunk
)


def encode_workspace_notification(
    writer: BinaryWriter, value: WorkspaceNotification
) -> None:
    """Encode one WorkspaceNotification."""
    if value.kind == "diagnostics":
        writer.write_unsigned(0)
        encode_diagnostics_notification(writer, value.diagnostics)
    elif value.kind == "messages":
        writer.write_unsigned(1)
        encode_message_notification(writer, value.messages)
    elif value.kind == "progress":
        writer.write_unsigned(2)
        encode_progress_notification(writer, value.progress)
    elif value.kind == "payloadChunk":
        writer.write_unsigned(3)
        destack._generated.protocol.payload.encode_payload_chunk_notification(
            writer, value.payload_chunk
        )
    else:
        raise SerdeError("unknown enum variant")


def decode_workspace_notification(reader: BinaryReader) -> WorkspaceNotification:
    """Decode one WorkspaceNotification."""
    variant = reader.read_number()

    if variant == 0:
        diagnostics = decode_diagnostics_notification(reader)

        return WorkspaceNotificationDiagnostics(diagnostics=diagnostics)
    elif variant == 1:
        messages = decode_message_notification(reader)

        return WorkspaceNotificationMessages(messages=messages)
    elif variant == 2:
        progress = decode_progress_notification(reader)

        return WorkspaceNotificationProgress(progress=progress)
    elif variant == 3:
        payload_chunk = (
            destack._generated.protocol.payload.decode_payload_chunk_notification(
                reader
            )
        )

        return WorkspaceNotificationPayloadChunk(payload_chunk=payload_chunk)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_workspace_notification(value: WorkspaceNotification) -> Json:
    """Return one JSON value for one WorkspaceNotification."""
    if value.kind == "diagnostics":
        return {
            "kind": "diagnostics",
            "diagnostics": to_json_diagnostics_notification(value.diagnostics),
        }
    elif value.kind == "messages":
        return {
            "kind": "messages",
            "messages": to_json_message_notification(value.messages),
        }
    elif value.kind == "progress":
        return {
            "kind": "progress",
            "progress": to_json_progress_notification(value.progress),
        }
    elif value.kind == "payloadChunk":
        return {
            "kind": "payloadChunk",
            "payload_chunk": destack._generated.protocol.payload.to_json_payload_chunk_notification(
                value.payload_chunk
            ),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_workspace_notification(value: Json) -> WorkspaceNotification:
    """Return one WorkspaceNotification from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "diagnostics":
        return WorkspaceNotificationDiagnostics(
            diagnostics=from_json_diagnostics_notification(
                json_field(object_, "diagnostics")
            )
        )
    elif kind == "messages":
        return WorkspaceNotificationMessages(
            messages=from_json_message_notification(json_field(object_, "messages"))
        )
    elif kind == "progress":
        return WorkspaceNotificationProgress(
            progress=from_json_progress_notification(json_field(object_, "progress"))
        )
    elif kind == "payloadChunk":
        return WorkspaceNotificationPayloadChunk(
            payload_chunk=destack._generated.protocol.payload.from_json_payload_chunk_notification(
                json_field(object_, "payload_chunk")
            )
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class DiagnosticsNotification:
    """Notification for diagnostics updates."""

    # root handle
    handle: destack._generated.protocol.root.RootId
    # diagnostics grouped by file
    diagnostics: Sequence[DiagnosticBatch]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_diagnostics_notification(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> DiagnosticsNotification:
        """Decode one DiagnosticsNotification."""
        return decode_diagnostics_notification(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_diagnostics_notification(self)

    @classmethod
    def from_json(cls, value: Json) -> DiagnosticsNotification:
        """Return one DiagnosticsNotification from one JSON value."""
        return from_json_diagnostics_notification(value)


def encode_diagnostics_notification(
    writer: BinaryWriter, value: DiagnosticsNotification
) -> None:
    """Encode one DiagnosticsNotification."""
    destack._generated.protocol.root.encode_root_id(writer, value.handle)
    writer.write_unsigned(len(value.diagnostics))
    for item_value_diagnostics_0 in value.diagnostics:
        encode_diagnostic_batch(writer, item_value_diagnostics_0)


def decode_diagnostics_notification(reader: BinaryReader) -> DiagnosticsNotification:
    """Decode one DiagnosticsNotification."""
    handle = destack._generated.protocol.root.decode_root_id(reader)
    diagnostics = [decode_diagnostic_batch(reader) for _ in range(reader.read_number())]

    return DiagnosticsNotification(
        handle=handle,
        diagnostics=diagnostics,
    )


def to_json_diagnostics_notification(value: DiagnosticsNotification) -> Json:
    """Return one JSON value for one DiagnosticsNotification."""
    return {
        "handle": destack._generated.protocol.root.to_json_root_id(value.handle),
        "diagnostics": [
            to_json_diagnostic_batch(item_0) for item_0 in value.diagnostics
        ],
    }


def from_json_diagnostics_notification(value: Json) -> DiagnosticsNotification:
    """Return one DiagnosticsNotification from one JSON value."""
    object_ = json_object(value)

    return DiagnosticsNotification(
        handle=destack._generated.protocol.root.from_json_root_id(
            json_field(object_, "handle")
        ),
        diagnostics=[
            from_json_diagnostic_batch(item_0)
            for item_0 in json_array(json_field(object_, "diagnostics"))
        ],
    )


@dataclass(frozen=True, slots=True)
class MessageNotification:
    """Notification for workspace messages."""

    # root handle
    handle: destack._generated.protocol.root.RootId
    # messages emitted by the workspace
    messages: Sequence[destack._generated.protocol.workspace.message.Message]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_message_notification(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> MessageNotification:
        """Decode one MessageNotification."""
        return decode_message_notification(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_message_notification(self)

    @classmethod
    def from_json(cls, value: Json) -> MessageNotification:
        """Return one MessageNotification from one JSON value."""
        return from_json_message_notification(value)


def encode_message_notification(
    writer: BinaryWriter, value: MessageNotification
) -> None:
    """Encode one MessageNotification."""
    destack._generated.protocol.root.encode_root_id(writer, value.handle)
    writer.write_unsigned(len(value.messages))
    for item_value_messages_0 in value.messages:
        destack._generated.protocol.workspace.message.encode_message(
            writer, item_value_messages_0
        )


def decode_message_notification(reader: BinaryReader) -> MessageNotification:
    """Decode one MessageNotification."""
    handle = destack._generated.protocol.root.decode_root_id(reader)
    messages = [
        destack._generated.protocol.workspace.message.decode_message(reader)
        for _ in range(reader.read_number())
    ]

    return MessageNotification(
        handle=handle,
        messages=messages,
    )


def to_json_message_notification(value: MessageNotification) -> Json:
    """Return one JSON value for one MessageNotification."""
    return {
        "handle": destack._generated.protocol.root.to_json_root_id(value.handle),
        "messages": [
            destack._generated.protocol.workspace.message.to_json_message(item_0)
            for item_0 in value.messages
        ],
    }


def from_json_message_notification(value: Json) -> MessageNotification:
    """Return one MessageNotification from one JSON value."""
    object_ = json_object(value)

    return MessageNotification(
        handle=destack._generated.protocol.root.from_json_root_id(
            json_field(object_, "handle")
        ),
        messages=[
            destack._generated.protocol.workspace.message.from_json_message(item_0)
            for item_0 in json_array(json_field(object_, "messages"))
        ],
    )


@dataclass(frozen=True, slots=True)
class ProgressNotification:
    """Progress notification payload."""

    # root handle
    handle: destack._generated.protocol.root.RootId
    # progress event payload
    event: destack._generated.protocol.workspace.command.common.ProgressEvent

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_progress_notification(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ProgressNotification:
        """Decode one ProgressNotification."""
        return decode_progress_notification(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_progress_notification(self)

    @classmethod
    def from_json(cls, value: Json) -> ProgressNotification:
        """Return one ProgressNotification from one JSON value."""
        return from_json_progress_notification(value)


def encode_progress_notification(
    writer: BinaryWriter, value: ProgressNotification
) -> None:
    """Encode one ProgressNotification."""
    destack._generated.protocol.root.encode_root_id(writer, value.handle)
    destack._generated.protocol.workspace.command.common.encode_progress_event(
        writer, value.event
    )


def decode_progress_notification(reader: BinaryReader) -> ProgressNotification:
    """Decode one ProgressNotification."""
    handle = destack._generated.protocol.root.decode_root_id(reader)
    event = destack._generated.protocol.workspace.command.common.decode_progress_event(
        reader
    )

    return ProgressNotification(
        handle=handle,
        event=event,
    )


def to_json_progress_notification(value: ProgressNotification) -> Json:
    """Return one JSON value for one ProgressNotification."""
    return {
        "handle": destack._generated.protocol.root.to_json_root_id(value.handle),
        "event": destack._generated.protocol.workspace.command.common.to_json_progress_event(
            value.event
        ),
    }


def from_json_progress_notification(value: Json) -> ProgressNotification:
    """Return one ProgressNotification from one JSON value."""
    object_ = json_object(value)

    return ProgressNotification(
        handle=destack._generated.protocol.root.from_json_root_id(
            json_field(object_, "handle")
        ),
        event=destack._generated.protocol.workspace.command.common.from_json_progress_event(
            json_field(object_, "event")
        ),
    )


__all__ = [
    "DiagnosticBatch",
    "encode_diagnostic_batch",
    "decode_diagnostic_batch",
    "to_json_diagnostic_batch",
    "from_json_diagnostic_batch",
    "WorkspaceNotification",
    "encode_workspace_notification",
    "decode_workspace_notification",
    "to_json_workspace_notification",
    "from_json_workspace_notification",
    "WorkspaceNotificationDiagnostics",
    "WorkspaceNotificationMessages",
    "WorkspaceNotificationProgress",
    "WorkspaceNotificationPayloadChunk",
    "DiagnosticsNotification",
    "encode_diagnostics_notification",
    "decode_diagnostics_notification",
    "to_json_diagnostics_notification",
    "from_json_diagnostics_notification",
    "MessageNotification",
    "encode_message_notification",
    "decode_message_notification",
    "to_json_message_notification",
    "from_json_message_notification",
    "ProgressNotification",
    "encode_progress_notification",
    "decode_progress_notification",
    "to_json_progress_notification",
    "from_json_progress_notification",
]
