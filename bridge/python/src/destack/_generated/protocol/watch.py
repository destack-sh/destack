# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes

import destack._generated.protocol.root
import destack._generated.protocol.workspace.message
import destack._generated.protocol.workspace.root

if TYPE_CHECKING:
    from destack._generated.protocol.root import (
        RootId,
    )

    from destack._generated.protocol.workspace.message import (
        UpdateBatch,
    )

    from destack._generated.protocol.workspace.root import (
        ReloadReason,
    )


@dataclass(frozen=True, slots=True)
class WatchStartRequest:
    """Request to start watching roots through a handle."""

    """Root handle."""
    handle: RootId
    """Roots watched through this handle."""
    roots: Sequence[str]
    """Watch start options."""
    options: WatchStartOptions


def encode_watch_start_request(writer: Writer, value: WatchStartRequest) -> None:
    destack._generated.protocol.root.encode_root_id(writer, value.handle)
    writer.write_unsigned(len(value.roots))
    for item_0 in value.roots:
        writer.write_string(item_0)
    encode_watch_start_options(writer, value.options)


def decode_watch_start_request(reader: Reader) -> WatchStartRequest:
    field_0 = destack._generated.protocol.root.decode_root_id(reader)
    field_1 = [reader.read_string() for _ in range(reader.read_number())]
    field_2 = decode_watch_start_options(reader)

    return WatchStartRequest(
        handle=field_0,
        roots=field_1,
        options=field_2,
    )


@dataclass(frozen=True, slots=True)
class WatchStartOptions:
    """Options for starting a workspace watch."""

    """Maximum time to coalesce events in milliseconds."""
    coalesce_window_ms: int
    """Maximum event and status count per batch."""
    max_batch_size: int


def encode_watch_start_options(writer: Writer, value: WatchStartOptions) -> None:
    writer.write_unsigned(value.coalesce_window_ms)
    writer.write_unsigned(value.max_batch_size)


def decode_watch_start_options(reader: Reader) -> WatchStartOptions:
    field_0 = reader.read_number()
    field_1 = reader.read_number()

    return WatchStartOptions(
        coalesce_window_ms=field_0,
        max_batch_size=field_1,
    )


@dataclass(frozen=True, slots=True)
class WatchNextRequest:
    """Request to receive and apply the next watch batch."""

    """Root handle."""
    handle: RootId


def encode_watch_next_request(writer: Writer, value: WatchNextRequest) -> None:
    destack._generated.protocol.root.encode_root_id(writer, value.handle)


def decode_watch_next_request(reader: Reader) -> WatchNextRequest:
    field_0 = destack._generated.protocol.root.decode_root_id(reader)

    return WatchNextRequest(
        handle=field_0,
    )


@dataclass(frozen=True, slots=True)
class WatchStopRequest:
    """Request to stop watching roots through a handle."""

    """Root handle."""
    handle: RootId


def encode_watch_stop_request(writer: Writer, value: WatchStopRequest) -> None:
    destack._generated.protocol.root.encode_root_id(writer, value.handle)


def decode_watch_stop_request(reader: Reader) -> WatchStopRequest:
    field_0 = destack._generated.protocol.root.decode_root_id(reader)

    return WatchStopRequest(
        handle=field_0,
    )


@dataclass(frozen=True, slots=True)
class WatchStartedResponse:
    """Response for watch start requests."""

    """Root handle."""
    handle: RootId


def encode_watch_started_response(writer: Writer, value: WatchStartedResponse) -> None:
    destack._generated.protocol.root.encode_root_id(writer, value.handle)


def decode_watch_started_response(reader: Reader) -> WatchStartedResponse:
    field_0 = destack._generated.protocol.root.decode_root_id(reader)

    return WatchStartedResponse(
        handle=field_0,
    )


@dataclass(frozen=True, slots=True)
class WatchBatchResponse:
    """Response for watch batch processing."""

    """Root handle."""
    handle: RootId
    """Watch batch received from the workspace watcher."""
    batch: WatchBatch | None
    """Updates produced by the batch."""
    updates: UpdateBatch


def encode_watch_batch_response(writer: Writer, value: WatchBatchResponse) -> None:
    destack._generated.protocol.root.encode_root_id(writer, value.handle)
    if value.batch is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_watch_batch(writer, value.batch)
    destack._generated.protocol.workspace.message.encode_update_batch(
        writer, value.updates
    )


def decode_watch_batch_response(reader: Reader) -> WatchBatchResponse:
    field_0 = destack._generated.protocol.root.decode_root_id(reader)
    field_1 = reader.read_option(lambda: decode_watch_batch(reader))
    field_2 = destack._generated.protocol.workspace.message.decode_update_batch(reader)

    return WatchBatchResponse(
        handle=field_0,
        batch=field_1,
        updates=field_2,
    )


@dataclass(frozen=True, slots=True)
class WatchBatch:
    """Watch batch payload used in the protocol."""

    """List of file events in the batch."""
    events: Sequence[WatchEvent]
    """Status updates emitted by the watcher."""
    status: Sequence[WatchStatus]
    """Whether overflow occurred."""
    overflowed: bool
    """Batch start timestamp in relative nanoseconds."""
    started_at_ns: int
    """Batch end timestamp in relative nanoseconds."""
    ended_at_ns: int


def encode_watch_batch(writer: Writer, value: WatchBatch) -> None:
    writer.write_unsigned(len(value.events))
    for item_0 in value.events:
        encode_watch_event(writer, item_0)
    writer.write_unsigned(len(value.status))
    for item_0 in value.status:
        encode_watch_status(writer, item_0)
    writer.write_bool(value.overflowed)
    writer.write_unsigned(value.started_at_ns)
    writer.write_unsigned(value.ended_at_ns)


def decode_watch_batch(reader: Reader) -> WatchBatch:
    field_0 = [decode_watch_event(reader) for _ in range(reader.read_number())]
    field_1 = [decode_watch_status(reader) for _ in range(reader.read_number())]
    field_2 = reader.read_bool()
    field_3 = reader.read_number()
    field_4 = reader.read_number()

    return WatchBatch(
        events=field_0,
        status=field_1,
        overflowed=field_2,
        started_at_ns=field_3,
        ended_at_ns=field_4,
    )


@dataclass(frozen=True, slots=True)
class WatchEvent:
    """Watch event payload."""

    """Event path."""
    path: str
    """Optional previous path for renames."""
    previous_path: str | None
    """Event kind."""
    kind: WatchEventKind


def encode_watch_event(writer: Writer, value: WatchEvent) -> None:
    writer.write_string(value.path)
    if value.previous_path is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.previous_path)
    encode_watch_event_kind(writer, value.kind)


def decode_watch_event(reader: Reader) -> WatchEvent:
    field_0 = reader.read_string()
    field_1 = reader.read_option(lambda: reader.read_string())
    field_2 = decode_watch_event_kind(reader)

    return WatchEvent(
        path=field_0,
        previous_path=field_1,
        kind=field_2,
    )


"""Watch event kind."""
WatchEventKind: TypeAlias = (
    Literal["created"]
    | Literal["modified"]
    | Literal["deleted"]
    | Literal["renamed"]
    | Literal["overflow"]
)


def encode_watch_event_kind(writer: Writer, value: WatchEventKind) -> None:
    if value == "created":
        writer.write_unsigned(0)
    elif value == "modified":
        writer.write_unsigned(1)
    elif value == "deleted":
        writer.write_unsigned(2)
    elif value == "renamed":
        writer.write_unsigned(3)
    elif value == "overflow":
        writer.write_unsigned(4)
    else:
        raise SerdeError("unknown enum variant")


def decode_watch_event_kind(reader: Reader) -> WatchEventKind:
    variant = reader.read_number()

    if variant == 0:
        return "created"
    elif variant == 1:
        return "modified"
    elif variant == 2:
        return "deleted"
    elif variant == 3:
        return "renamed"
    elif variant == 4:
        return "overflow"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


@dataclass(frozen=True, slots=True)
class WatchStatusReady:
    """Watcher is ready."""

    roots: Sequence[str]
    kind: Literal["ready"] = "ready"


@dataclass(frozen=True, slots=True)
class WatchStatusReloadRequested:
    """Watcher requests a filesystem reload."""

    """Watch roots for the reload."""
    roots: Sequence[str]
    """Reload reason."""
    reason: ReloadReason
    kind: Literal["reloadRequested"] = "reloadRequested"


@dataclass(frozen=True, slots=True)
class WatchStatusError:
    """Watcher encountered an error."""

    message: str
    kind: Literal["error"] = "error"


@dataclass(frozen=True, slots=True)
class WatchStatusStopped:
    """Watcher stopped."""

    kind: Literal["stopped"] = "stopped"


"""Watch status update."""
WatchStatus: TypeAlias = (
    WatchStatusReady
    | WatchStatusReloadRequested
    | WatchStatusError
    | WatchStatusStopped
)


def encode_watch_status(writer: Writer, value: WatchStatus) -> None:
    if value.kind == "ready":
        writer.write_unsigned(0)
        writer.write_unsigned(len(value.roots))
        for item_0 in value.roots:
            writer.write_string(item_0)
    elif value.kind == "reloadRequested":
        writer.write_unsigned(1)
        writer.write_unsigned(len(value.roots))
        for item_0 in value.roots:
            writer.write_string(item_0)
        destack._generated.protocol.workspace.root.encode_reload_reason(
            writer, value.reason
        )
    elif value.kind == "error":
        writer.write_unsigned(2)
        writer.write_string(value.message)
    elif value.kind == "stopped":
        writer.write_unsigned(3)
    else:
        raise SerdeError("unknown enum variant")


def decode_watch_status(reader: Reader) -> WatchStatus:
    variant = reader.read_number()

    if variant == 0:
        field_0 = [reader.read_string() for _ in range(reader.read_number())]

        return WatchStatusReady(
            roots=field_0,
        )
    elif variant == 1:
        field_0 = [reader.read_string() for _ in range(reader.read_number())]
        field_1 = destack._generated.protocol.workspace.root.decode_reload_reason(
            reader
        )

        return WatchStatusReloadRequested(
            roots=field_0,
            reason=field_1,
        )
    elif variant == 2:
        field_0 = reader.read_string()

        return WatchStatusError(
            message=field_0,
        )
    elif variant == 3:
        return WatchStatusStopped()
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


@dataclass(frozen=True, slots=True)
class WatchStoppedResponse:
    """Response for watch stop requests."""

    """Root handle."""
    handle: RootId


def encode_watch_stopped_response(writer: Writer, value: WatchStoppedResponse) -> None:
    destack._generated.protocol.root.encode_root_id(writer, value.handle)


def decode_watch_stopped_response(reader: Reader) -> WatchStoppedResponse:
    field_0 = destack._generated.protocol.root.decode_root_id(reader)

    return WatchStoppedResponse(
        handle=field_0,
    )


__all__ = [
    "WatchStartRequest",
    "encode_watch_start_request",
    "decode_watch_start_request",
    "WatchStartOptions",
    "encode_watch_start_options",
    "decode_watch_start_options",
    "WatchNextRequest",
    "encode_watch_next_request",
    "decode_watch_next_request",
    "WatchStopRequest",
    "encode_watch_stop_request",
    "decode_watch_stop_request",
    "WatchStartedResponse",
    "encode_watch_started_response",
    "decode_watch_started_response",
    "WatchBatchResponse",
    "encode_watch_batch_response",
    "decode_watch_batch_response",
    "WatchBatch",
    "encode_watch_batch",
    "decode_watch_batch",
    "WatchEvent",
    "encode_watch_event",
    "decode_watch_event",
    "WatchEventKind",
    "encode_watch_event_kind",
    "decode_watch_event_kind",
    "WatchStatus",
    "encode_watch_status",
    "decode_watch_status",
    "WatchStatusReady",
    "WatchStatusReloadRequested",
    "WatchStatusError",
    "WatchStatusStopped",
    "WatchStoppedResponse",
    "encode_watch_stopped_response",
    "decode_watch_stopped_response",
]
