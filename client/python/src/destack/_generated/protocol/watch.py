# generated client target, do not edit

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
    json_bool,
    json_field,
    json_int,
    json_object,
    json_optional,
    json_string,
)

import destack._generated.protocol.root
import destack._generated.protocol.workspace.message
import destack._generated.protocol.workspace.root


@dataclass(frozen=True, slots=True)
class WatchStartRequest:
    """Request to start watching roots through a handle."""

    # root handle
    handle: destack._generated.protocol.root.RootId
    # roots watched through this handle
    roots: Sequence[str]
    # watch start options
    options: WatchStartOptions

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_watch_start_request(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> WatchStartRequest:
        """Decode one WatchStartRequest."""
        return decode_watch_start_request(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_watch_start_request(self)

    @classmethod
    def from_json(cls, value: Json) -> WatchStartRequest:
        """Return one WatchStartRequest from one JSON value."""
        return from_json_watch_start_request(value)


def encode_watch_start_request(writer: BinaryWriter, value: WatchStartRequest) -> None:
    """Encode one WatchStartRequest."""
    destack._generated.protocol.root.encode_root_id(writer, value.handle)
    writer.write_unsigned(len(value.roots))
    for item_value_roots_0 in value.roots:
        writer.write_string(item_value_roots_0)
    encode_watch_start_options(writer, value.options)


def decode_watch_start_request(reader: BinaryReader) -> WatchStartRequest:
    """Decode one WatchStartRequest."""
    handle = destack._generated.protocol.root.decode_root_id(reader)
    roots = [reader.read_string() for _ in range(reader.read_number())]
    options = decode_watch_start_options(reader)

    return WatchStartRequest(
        handle=handle,
        roots=roots,
        options=options,
    )


def to_json_watch_start_request(value: WatchStartRequest) -> Json:
    """Return one JSON value for one WatchStartRequest."""
    return {
        "handle": destack._generated.protocol.root.to_json_root_id(value.handle),
        "roots": [item_0 for item_0 in value.roots],
        "options": to_json_watch_start_options(value.options),
    }


def from_json_watch_start_request(value: Json) -> WatchStartRequest:
    """Return one WatchStartRequest from one JSON value."""
    object_ = json_object(value)

    return WatchStartRequest(
        handle=destack._generated.protocol.root.from_json_root_id(
            json_field(object_, "handle")
        ),
        roots=[
            json_string(item_0) for item_0 in json_array(json_field(object_, "roots"))
        ],
        options=from_json_watch_start_options(json_field(object_, "options")),
    )


@dataclass(frozen=True, slots=True)
class WatchStartOptions:
    """Options for starting a workspace watch."""

    # maximum time to coalesce events in milliseconds
    coalesce_window_ms: int
    # maximum event and status count per batch
    max_batch_size: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_watch_start_options(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> WatchStartOptions:
        """Decode one WatchStartOptions."""
        return decode_watch_start_options(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_watch_start_options(self)

    @classmethod
    def from_json(cls, value: Json) -> WatchStartOptions:
        """Return one WatchStartOptions from one JSON value."""
        return from_json_watch_start_options(value)


def encode_watch_start_options(writer: BinaryWriter, value: WatchStartOptions) -> None:
    """Encode one WatchStartOptions."""
    writer.write_unsigned(value.coalesce_window_ms)
    writer.write_unsigned(value.max_batch_size)


def decode_watch_start_options(reader: BinaryReader) -> WatchStartOptions:
    """Decode one WatchStartOptions."""
    coalesce_window_ms = reader.read_number()
    max_batch_size = reader.read_number()

    return WatchStartOptions(
        coalesce_window_ms=coalesce_window_ms,
        max_batch_size=max_batch_size,
    )


def to_json_watch_start_options(value: WatchStartOptions) -> Json:
    """Return one JSON value for one WatchStartOptions."""
    return {
        "coalesceWindowMs": value.coalesce_window_ms,
        "maxBatchSize": value.max_batch_size,
    }


def from_json_watch_start_options(value: Json) -> WatchStartOptions:
    """Return one WatchStartOptions from one JSON value."""
    object_ = json_object(value)

    return WatchStartOptions(
        coalesce_window_ms=json_int(json_field(object_, "coalesceWindowMs")),
        max_batch_size=json_int(json_field(object_, "maxBatchSize")),
    )


@dataclass(frozen=True, slots=True)
class WatchNextRequest:
    """Request to receive and apply the next watch batch."""

    # root handle
    handle: destack._generated.protocol.root.RootId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_watch_next_request(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> WatchNextRequest:
        """Decode one WatchNextRequest."""
        return decode_watch_next_request(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_watch_next_request(self)

    @classmethod
    def from_json(cls, value: Json) -> WatchNextRequest:
        """Return one WatchNextRequest from one JSON value."""
        return from_json_watch_next_request(value)


def encode_watch_next_request(writer: BinaryWriter, value: WatchNextRequest) -> None:
    """Encode one WatchNextRequest."""
    destack._generated.protocol.root.encode_root_id(writer, value.handle)


def decode_watch_next_request(reader: BinaryReader) -> WatchNextRequest:
    """Decode one WatchNextRequest."""
    handle = destack._generated.protocol.root.decode_root_id(reader)

    return WatchNextRequest(
        handle=handle,
    )


def to_json_watch_next_request(value: WatchNextRequest) -> Json:
    """Return one JSON value for one WatchNextRequest."""
    return {
        "handle": destack._generated.protocol.root.to_json_root_id(value.handle),
    }


def from_json_watch_next_request(value: Json) -> WatchNextRequest:
    """Return one WatchNextRequest from one JSON value."""
    object_ = json_object(value)

    return WatchNextRequest(
        handle=destack._generated.protocol.root.from_json_root_id(
            json_field(object_, "handle")
        ),
    )


@dataclass(frozen=True, slots=True)
class WatchStopRequest:
    """Request to stop watching roots through a handle."""

    # root handle
    handle: destack._generated.protocol.root.RootId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_watch_stop_request(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> WatchStopRequest:
        """Decode one WatchStopRequest."""
        return decode_watch_stop_request(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_watch_stop_request(self)

    @classmethod
    def from_json(cls, value: Json) -> WatchStopRequest:
        """Return one WatchStopRequest from one JSON value."""
        return from_json_watch_stop_request(value)


def encode_watch_stop_request(writer: BinaryWriter, value: WatchStopRequest) -> None:
    """Encode one WatchStopRequest."""
    destack._generated.protocol.root.encode_root_id(writer, value.handle)


def decode_watch_stop_request(reader: BinaryReader) -> WatchStopRequest:
    """Decode one WatchStopRequest."""
    handle = destack._generated.protocol.root.decode_root_id(reader)

    return WatchStopRequest(
        handle=handle,
    )


def to_json_watch_stop_request(value: WatchStopRequest) -> Json:
    """Return one JSON value for one WatchStopRequest."""
    return {
        "handle": destack._generated.protocol.root.to_json_root_id(value.handle),
    }


def from_json_watch_stop_request(value: Json) -> WatchStopRequest:
    """Return one WatchStopRequest from one JSON value."""
    object_ = json_object(value)

    return WatchStopRequest(
        handle=destack._generated.protocol.root.from_json_root_id(
            json_field(object_, "handle")
        ),
    )


@dataclass(frozen=True, slots=True)
class WatchStartedResponse:
    """Response for watch start requests."""

    # root handle
    handle: destack._generated.protocol.root.RootId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_watch_started_response(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> WatchStartedResponse:
        """Decode one WatchStartedResponse."""
        return decode_watch_started_response(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_watch_started_response(self)

    @classmethod
    def from_json(cls, value: Json) -> WatchStartedResponse:
        """Return one WatchStartedResponse from one JSON value."""
        return from_json_watch_started_response(value)


def encode_watch_started_response(
    writer: BinaryWriter, value: WatchStartedResponse
) -> None:
    """Encode one WatchStartedResponse."""
    destack._generated.protocol.root.encode_root_id(writer, value.handle)


def decode_watch_started_response(reader: BinaryReader) -> WatchStartedResponse:
    """Decode one WatchStartedResponse."""
    handle = destack._generated.protocol.root.decode_root_id(reader)

    return WatchStartedResponse(
        handle=handle,
    )


def to_json_watch_started_response(value: WatchStartedResponse) -> Json:
    """Return one JSON value for one WatchStartedResponse."""
    return {
        "handle": destack._generated.protocol.root.to_json_root_id(value.handle),
    }


def from_json_watch_started_response(value: Json) -> WatchStartedResponse:
    """Return one WatchStartedResponse from one JSON value."""
    object_ = json_object(value)

    return WatchStartedResponse(
        handle=destack._generated.protocol.root.from_json_root_id(
            json_field(object_, "handle")
        ),
    )


@dataclass(frozen=True, slots=True)
class WatchBatchResponse:
    """Response for watch batch processing."""

    # root handle
    handle: destack._generated.protocol.root.RootId
    # watch batch received from the workspace watcher
    batch: WatchBatch | None
    # updates produced by the batch
    updates: destack._generated.protocol.workspace.message.UpdateBatch

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_watch_batch_response(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> WatchBatchResponse:
        """Decode one WatchBatchResponse."""
        return decode_watch_batch_response(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_watch_batch_response(self)

    @classmethod
    def from_json(cls, value: Json) -> WatchBatchResponse:
        """Return one WatchBatchResponse from one JSON value."""
        return from_json_watch_batch_response(value)


def encode_watch_batch_response(
    writer: BinaryWriter, value: WatchBatchResponse
) -> None:
    """Encode one WatchBatchResponse."""
    destack._generated.protocol.root.encode_root_id(writer, value.handle)
    if value.batch is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_watch_batch(writer, value.batch)
    destack._generated.protocol.workspace.message.encode_update_batch(
        writer, value.updates
    )


def decode_watch_batch_response(reader: BinaryReader) -> WatchBatchResponse:
    """Decode one WatchBatchResponse."""
    handle = destack._generated.protocol.root.decode_root_id(reader)
    batch = reader.read_option(lambda: decode_watch_batch(reader))
    updates = destack._generated.protocol.workspace.message.decode_update_batch(reader)

    return WatchBatchResponse(
        handle=handle,
        batch=batch,
        updates=updates,
    )


def to_json_watch_batch_response(value: WatchBatchResponse) -> Json:
    """Return one JSON value for one WatchBatchResponse."""
    return {
        "handle": destack._generated.protocol.root.to_json_root_id(value.handle),
        **({} if value.batch is None else {"batch": to_json_watch_batch(value.batch)}),
        "updates": destack._generated.protocol.workspace.message.to_json_update_batch(
            value.updates
        ),
    }


def from_json_watch_batch_response(value: Json) -> WatchBatchResponse:
    """Return one WatchBatchResponse from one JSON value."""
    object_ = json_object(value)

    return WatchBatchResponse(
        handle=destack._generated.protocol.root.from_json_root_id(
            json_field(object_, "handle")
        ),
        batch=json_optional(
            object_, "batch", lambda value: from_json_watch_batch(value)
        ),
        updates=destack._generated.protocol.workspace.message.from_json_update_batch(
            json_field(object_, "updates")
        ),
    )


@dataclass(frozen=True, slots=True)
class WatchBatch:
    """Watch batch payload used in the protocol."""

    # list of file events in the batch
    events: Sequence[WatchEvent]
    # status updates emitted by the watcher
    status: Sequence[WatchStatus]
    # whether overflow occurred
    overflowed: bool
    # batch start timestamp in relative nanoseconds
    started_at_ns: int
    # batch end timestamp in relative nanoseconds
    ended_at_ns: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_watch_batch(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> WatchBatch:
        """Decode one WatchBatch."""
        return decode_watch_batch(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_watch_batch(self)

    @classmethod
    def from_json(cls, value: Json) -> WatchBatch:
        """Return one WatchBatch from one JSON value."""
        return from_json_watch_batch(value)


def encode_watch_batch(writer: BinaryWriter, value: WatchBatch) -> None:
    """Encode one WatchBatch."""
    writer.write_unsigned(len(value.events))
    for item_value_events_0 in value.events:
        encode_watch_event(writer, item_value_events_0)
    writer.write_unsigned(len(value.status))
    for item_value_status_0 in value.status:
        encode_watch_status(writer, item_value_status_0)
    writer.write_bool(value.overflowed)
    writer.write_unsigned(value.started_at_ns)
    writer.write_unsigned(value.ended_at_ns)


def decode_watch_batch(reader: BinaryReader) -> WatchBatch:
    """Decode one WatchBatch."""
    events = [decode_watch_event(reader) for _ in range(reader.read_number())]
    status = [decode_watch_status(reader) for _ in range(reader.read_number())]
    overflowed = reader.read_bool()
    started_at_ns = reader.read_number()
    ended_at_ns = reader.read_number()

    return WatchBatch(
        events=events,
        status=status,
        overflowed=overflowed,
        started_at_ns=started_at_ns,
        ended_at_ns=ended_at_ns,
    )


def to_json_watch_batch(value: WatchBatch) -> Json:
    """Return one JSON value for one WatchBatch."""
    return {
        "events": [to_json_watch_event(item_0) for item_0 in value.events],
        "status": [to_json_watch_status(item_0) for item_0 in value.status],
        "overflowed": value.overflowed,
        "startedAtNs": value.started_at_ns,
        "endedAtNs": value.ended_at_ns,
    }


def from_json_watch_batch(value: Json) -> WatchBatch:
    """Return one WatchBatch from one JSON value."""
    object_ = json_object(value)

    return WatchBatch(
        events=[
            from_json_watch_event(item_0)
            for item_0 in json_array(json_field(object_, "events"))
        ],
        status=[
            from_json_watch_status(item_0)
            for item_0 in json_array(json_field(object_, "status"))
        ],
        overflowed=json_bool(json_field(object_, "overflowed")),
        started_at_ns=json_int(json_field(object_, "startedAtNs")),
        ended_at_ns=json_int(json_field(object_, "endedAtNs")),
    )


@dataclass(frozen=True, slots=True)
class WatchEvent:
    """Watch event payload."""

    # event path
    path: str
    # optional previous path for renames
    previous_path: str | None
    # event kind
    kind: WatchEventKind

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_watch_event(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> WatchEvent:
        """Decode one WatchEvent."""
        return decode_watch_event(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_watch_event(self)

    @classmethod
    def from_json(cls, value: Json) -> WatchEvent:
        """Return one WatchEvent from one JSON value."""
        return from_json_watch_event(value)


def encode_watch_event(writer: BinaryWriter, value: WatchEvent) -> None:
    """Encode one WatchEvent."""
    writer.write_string(value.path)
    if value.previous_path is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.previous_path)
    encode_watch_event_kind(writer, value.kind)


def decode_watch_event(reader: BinaryReader) -> WatchEvent:
    """Decode one WatchEvent."""
    path = reader.read_string()
    previous_path = reader.read_option(lambda: reader.read_string())
    kind = decode_watch_event_kind(reader)

    return WatchEvent(
        path=path,
        previous_path=previous_path,
        kind=kind,
    )


def to_json_watch_event(value: WatchEvent) -> Json:
    """Return one JSON value for one WatchEvent."""
    return {
        "path": value.path,
        **(
            {} if value.previous_path is None else {"previousPath": value.previous_path}
        ),
        "kind": to_json_watch_event_kind(value.kind),
    }


def from_json_watch_event(value: Json) -> WatchEvent:
    """Return one WatchEvent from one JSON value."""
    object_ = json_object(value)

    return WatchEvent(
        path=json_string(json_field(object_, "path")),
        previous_path=json_optional(
            object_, "previousPath", lambda value: json_string(value)
        ),
        kind=from_json_watch_event_kind(json_field(object_, "kind")),
    )


"""Watch event kind."""
WatchEventKind: typing.TypeAlias = (
    typing.Literal["created"]
    | typing.Literal["modified"]
    | typing.Literal["deleted"]
    | typing.Literal["renamed"]
    | typing.Literal["overflow"]
)


def encode_watch_event_kind(writer: BinaryWriter, value: WatchEventKind) -> None:
    """Encode one WatchEventKind."""
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


def decode_watch_event_kind(reader: BinaryReader) -> WatchEventKind:
    """Decode one WatchEventKind."""
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


def to_json_watch_event_kind(value: WatchEventKind) -> Json:
    """Return one JSON value for one WatchEventKind."""
    return value


def from_json_watch_event_kind(value: Json) -> WatchEventKind:
    """Return one WatchEventKind from one JSON value."""
    variant = json_string(value)

    if variant == "created":
        return "created"
    elif variant == "modified":
        return "modified"
    elif variant == "deleted":
        return "deleted"
    elif variant == "renamed":
        return "renamed"
    elif variant == "overflow":
        return "overflow"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class WatchStatusReady:
    """Watcher is ready."""

    roots: Sequence[str]
    kind: typing.Literal["ready"] = "ready"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_watch_status(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_watch_status(self)


@dataclass(frozen=True, slots=True)
class WatchStatusReloadRequested:
    """Watcher requests a filesystem reload."""

    # watch roots for the reload
    roots: Sequence[str]
    # reload reason
    reason: destack._generated.protocol.workspace.root.ReloadReason
    kind: typing.Literal["reloadRequested"] = "reloadRequested"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_watch_status(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_watch_status(self)


@dataclass(frozen=True, slots=True)
class WatchStatusError:
    """Watcher encountered an error."""

    message: str
    kind: typing.Literal["error"] = "error"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_watch_status(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_watch_status(self)


@dataclass(frozen=True, slots=True)
class WatchStatusStopped:
    """Watcher stopped."""

    kind: typing.Literal["stopped"] = "stopped"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_watch_status(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_watch_status(self)


"""Watch status update."""
WatchStatus: typing.TypeAlias = (
    WatchStatusReady
    | WatchStatusReloadRequested
    | WatchStatusError
    | WatchStatusStopped
)


def encode_watch_status(writer: BinaryWriter, value: WatchStatus) -> None:
    """Encode one WatchStatus."""
    if value.kind == "ready":
        writer.write_unsigned(0)
        writer.write_unsigned(len(value.roots))
        for item_value_roots_0 in value.roots:
            writer.write_string(item_value_roots_0)
    elif value.kind == "reloadRequested":
        writer.write_unsigned(1)
        writer.write_unsigned(len(value.roots))
        for item_value_roots_0 in value.roots:
            writer.write_string(item_value_roots_0)
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


def decode_watch_status(reader: BinaryReader) -> WatchStatus:
    """Decode one WatchStatus."""
    variant = reader.read_number()

    if variant == 0:
        roots = [reader.read_string() for _ in range(reader.read_number())]

        return WatchStatusReady(
            roots=roots,
        )
    elif variant == 1:
        roots = [reader.read_string() for _ in range(reader.read_number())]
        reason = destack._generated.protocol.workspace.root.decode_reload_reason(reader)

        return WatchStatusReloadRequested(
            roots=roots,
            reason=reason,
        )
    elif variant == 2:
        message = reader.read_string()

        return WatchStatusError(
            message=message,
        )
    elif variant == 3:
        return WatchStatusStopped()
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_watch_status(value: WatchStatus) -> Json:
    """Return one JSON value for one WatchStatus."""
    if value.kind == "ready":
        return {
            "kind": "ready",
            "roots": [item_0 for item_0 in value.roots],
        }
    elif value.kind == "reloadRequested":
        return {
            "kind": "reloadRequested",
            "roots": [item_0 for item_0 in value.roots],
            "reason": destack._generated.protocol.workspace.root.to_json_reload_reason(
                value.reason
            ),
        }
    elif value.kind == "error":
        return {
            "kind": "error",
            "message": value.message,
        }
    elif value.kind == "stopped":
        return {
            "kind": "stopped",
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_watch_status(value: Json) -> WatchStatus:
    """Return one WatchStatus from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "ready":
        return WatchStatusReady(
            roots=[
                json_string(item_0)
                for item_0 in json_array(json_field(object_, "roots"))
            ],
        )
    elif kind == "reloadRequested":
        return WatchStatusReloadRequested(
            roots=[
                json_string(item_0)
                for item_0 in json_array(json_field(object_, "roots"))
            ],
            reason=destack._generated.protocol.workspace.root.from_json_reload_reason(
                json_field(object_, "reason")
            ),
        )
    elif kind == "error":
        return WatchStatusError(
            message=json_string(json_field(object_, "message")),
        )
    elif kind == "stopped":
        return WatchStatusStopped()
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class WatchStoppedResponse:
    """Response for watch stop requests."""

    # root handle
    handle: destack._generated.protocol.root.RootId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_watch_stopped_response(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> WatchStoppedResponse:
        """Decode one WatchStoppedResponse."""
        return decode_watch_stopped_response(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_watch_stopped_response(self)

    @classmethod
    def from_json(cls, value: Json) -> WatchStoppedResponse:
        """Return one WatchStoppedResponse from one JSON value."""
        return from_json_watch_stopped_response(value)


def encode_watch_stopped_response(
    writer: BinaryWriter, value: WatchStoppedResponse
) -> None:
    """Encode one WatchStoppedResponse."""
    destack._generated.protocol.root.encode_root_id(writer, value.handle)


def decode_watch_stopped_response(reader: BinaryReader) -> WatchStoppedResponse:
    """Decode one WatchStoppedResponse."""
    handle = destack._generated.protocol.root.decode_root_id(reader)

    return WatchStoppedResponse(
        handle=handle,
    )


def to_json_watch_stopped_response(value: WatchStoppedResponse) -> Json:
    """Return one JSON value for one WatchStoppedResponse."""
    return {
        "handle": destack._generated.protocol.root.to_json_root_id(value.handle),
    }


def from_json_watch_stopped_response(value: Json) -> WatchStoppedResponse:
    """Return one WatchStoppedResponse from one JSON value."""
    object_ = json_object(value)

    return WatchStoppedResponse(
        handle=destack._generated.protocol.root.from_json_root_id(
            json_field(object_, "handle")
        ),
    )


__all__ = [
    "WatchStartRequest",
    "encode_watch_start_request",
    "decode_watch_start_request",
    "to_json_watch_start_request",
    "from_json_watch_start_request",
    "WatchStartOptions",
    "encode_watch_start_options",
    "decode_watch_start_options",
    "to_json_watch_start_options",
    "from_json_watch_start_options",
    "WatchNextRequest",
    "encode_watch_next_request",
    "decode_watch_next_request",
    "to_json_watch_next_request",
    "from_json_watch_next_request",
    "WatchStopRequest",
    "encode_watch_stop_request",
    "decode_watch_stop_request",
    "to_json_watch_stop_request",
    "from_json_watch_stop_request",
    "WatchStartedResponse",
    "encode_watch_started_response",
    "decode_watch_started_response",
    "to_json_watch_started_response",
    "from_json_watch_started_response",
    "WatchBatchResponse",
    "encode_watch_batch_response",
    "decode_watch_batch_response",
    "to_json_watch_batch_response",
    "from_json_watch_batch_response",
    "WatchBatch",
    "encode_watch_batch",
    "decode_watch_batch",
    "to_json_watch_batch",
    "from_json_watch_batch",
    "WatchEvent",
    "encode_watch_event",
    "decode_watch_event",
    "to_json_watch_event",
    "from_json_watch_event",
    "WatchEventKind",
    "encode_watch_event_kind",
    "decode_watch_event_kind",
    "to_json_watch_event_kind",
    "from_json_watch_event_kind",
    "WatchStatus",
    "encode_watch_status",
    "decode_watch_status",
    "to_json_watch_status",
    "from_json_watch_status",
    "WatchStatusReady",
    "WatchStatusReloadRequested",
    "WatchStatusError",
    "WatchStatusStopped",
    "WatchStoppedResponse",
    "encode_watch_stopped_response",
    "decode_watch_stopped_response",
    "to_json_watch_stopped_response",
    "from_json_watch_stopped_response",
]
