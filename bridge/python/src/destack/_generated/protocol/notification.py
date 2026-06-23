# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes

import destack._generated.protocol.payload
import destack._generated.protocol.root
import destack._generated.protocol.source.diagnostic.diagnostic
import destack._generated.protocol.source.file.model.file
import destack._generated.protocol.workspace.command.common
import destack._generated.protocol.workspace.message

if TYPE_CHECKING:
    from destack._generated.protocol.payload import (
        PayloadChunkNotification,
    )

    from destack._generated.protocol.root import (
        RootId,
    )

    from destack._generated.protocol.source.diagnostic.diagnostic import (
        Diagnostic,
    )

    from destack._generated.protocol.source.file.model.file import (
        FileId,
    )

    from destack._generated.protocol.workspace.command.common import (
        ProgressEvent,
    )

    from destack._generated.protocol.workspace.message import (
        Message,
    )


@dataclass(frozen=True, slots=True)
class DiagnosticBatch:
    """Diagnostic batch for notifications."""

    """The file id for these diagnostics."""
    file_id: FileId
    """Diagnostics for the file."""
    diagnostics: Sequence[Diagnostic]


def encode_diagnostic_batch(writer: Writer, value: DiagnosticBatch) -> None:
    destack._generated.protocol.source.file.model.file.encode_file_id(
        writer, value.file_id
    )
    writer.write_unsigned(len(value.diagnostics))
    for item_0 in value.diagnostics:
        destack._generated.protocol.source.diagnostic.diagnostic.encode_diagnostic(
            writer, item_0
        )


def decode_diagnostic_batch(reader: Reader) -> DiagnosticBatch:
    field_0 = destack._generated.protocol.source.file.model.file.decode_file_id(reader)
    field_1 = [
        destack._generated.protocol.source.diagnostic.diagnostic.decode_diagnostic(
            reader
        )
        for _ in range(reader.read_number())
    ]

    return DiagnosticBatch(
        file_id=field_0,
        diagnostics=field_1,
    )


@dataclass(frozen=True, slots=True)
class WorkspaceNotificationDiagnostics:
    """Publish diagnostics for a root."""

    diagnostics: DiagnosticsNotification
    kind: Literal["diagnostics"] = "diagnostics"


@dataclass(frozen=True, slots=True)
class WorkspaceNotificationMessages:
    """Publish workspace messages."""

    messages: MessageNotification
    kind: Literal["messages"] = "messages"


@dataclass(frozen=True, slots=True)
class WorkspaceNotificationProgress:
    """Publish progress updates."""

    progress: ProgressNotification
    kind: Literal["progress"] = "progress"


@dataclass(frozen=True, slots=True)
class WorkspaceNotificationPayloadChunk:
    """Publish chunked payload data."""

    payload_chunk: PayloadChunkNotification
    kind: Literal["payloadChunk"] = "payloadChunk"


"""Notifications emitted by the workspace protocol."""
WorkspaceNotification: TypeAlias = (
    WorkspaceNotificationDiagnostics
    | WorkspaceNotificationMessages
    | WorkspaceNotificationProgress
    | WorkspaceNotificationPayloadChunk
)


def encode_workspace_notification(writer: Writer, value: WorkspaceNotification) -> None:
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


def decode_workspace_notification(reader: Reader) -> WorkspaceNotification:
    variant = reader.read_number()

    if variant == 0:
        return WorkspaceNotificationDiagnostics(
            diagnostics=decode_diagnostics_notification(reader)
        )
    elif variant == 1:
        return WorkspaceNotificationMessages(
            messages=decode_message_notification(reader)
        )
    elif variant == 2:
        return WorkspaceNotificationProgress(
            progress=decode_progress_notification(reader)
        )
    elif variant == 3:
        return WorkspaceNotificationPayloadChunk(
            payload_chunk=destack._generated.protocol.payload.decode_payload_chunk_notification(
                reader
            )
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


@dataclass(frozen=True, slots=True)
class DiagnosticsNotification:
    """Notification for diagnostics updates."""

    """Root handle."""
    handle: RootId
    """Diagnostics grouped by file."""
    diagnostics: Sequence[DiagnosticBatch]


def encode_diagnostics_notification(
    writer: Writer, value: DiagnosticsNotification
) -> None:
    destack._generated.protocol.root.encode_root_id(writer, value.handle)
    writer.write_unsigned(len(value.diagnostics))
    for item_0 in value.diagnostics:
        encode_diagnostic_batch(writer, item_0)


def decode_diagnostics_notification(reader: Reader) -> DiagnosticsNotification:
    field_0 = destack._generated.protocol.root.decode_root_id(reader)
    field_1 = [decode_diagnostic_batch(reader) for _ in range(reader.read_number())]

    return DiagnosticsNotification(
        handle=field_0,
        diagnostics=field_1,
    )


@dataclass(frozen=True, slots=True)
class MessageNotification:
    """Notification for workspace messages."""

    """Root handle."""
    handle: RootId
    """Messages emitted by the workspace."""
    messages: Sequence[Message]


def encode_message_notification(writer: Writer, value: MessageNotification) -> None:
    destack._generated.protocol.root.encode_root_id(writer, value.handle)
    writer.write_unsigned(len(value.messages))
    for item_0 in value.messages:
        destack._generated.protocol.workspace.message.encode_message(writer, item_0)


def decode_message_notification(reader: Reader) -> MessageNotification:
    field_0 = destack._generated.protocol.root.decode_root_id(reader)
    field_1 = [
        destack._generated.protocol.workspace.message.decode_message(reader)
        for _ in range(reader.read_number())
    ]

    return MessageNotification(
        handle=field_0,
        messages=field_1,
    )


@dataclass(frozen=True, slots=True)
class ProgressNotification:
    """Progress notification payload."""

    """Root handle."""
    handle: RootId
    """Progress event payload."""
    event: ProgressEvent


def encode_progress_notification(writer: Writer, value: ProgressNotification) -> None:
    destack._generated.protocol.root.encode_root_id(writer, value.handle)
    destack._generated.protocol.workspace.command.common.encode_progress_event(
        writer, value.event
    )


def decode_progress_notification(reader: Reader) -> ProgressNotification:
    field_0 = destack._generated.protocol.root.decode_root_id(reader)
    field_1 = (
        destack._generated.protocol.workspace.command.common.decode_progress_event(
            reader
        )
    )

    return ProgressNotification(
        handle=field_0,
        event=field_1,
    )


__all__ = [
    "DiagnosticBatch",
    "encode_diagnostic_batch",
    "decode_diagnostic_batch",
    "WorkspaceNotification",
    "encode_workspace_notification",
    "decode_workspace_notification",
    "WorkspaceNotificationDiagnostics",
    "WorkspaceNotificationMessages",
    "WorkspaceNotificationProgress",
    "WorkspaceNotificationPayloadChunk",
    "DiagnosticsNotification",
    "encode_diagnostics_notification",
    "decode_diagnostics_notification",
    "MessageNotification",
    "encode_message_notification",
    "decode_message_notification",
    "ProgressNotification",
    "encode_progress_notification",
    "decode_progress_notification",
]
