# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_field,
    json_int,
    json_object,
    json_optional,
    json_string,
)

import destack._generated.repository.config.runtime.execution


@dataclass(frozen=True, slots=True)
class TraceOptions:
    """Runtime trace configuration."""

    # the configured restore contract
    restore: RestoreMode
    # base path for trace logs
    path: str | None
    # template for auto-generated log file names
    template: str | None
    # chunk size in megabytes for log rotation
    chunk_size_mb: int | None
    # trace payload selection for recorded binding calls
    payload: destack._generated.repository.config.runtime.execution.ReplayPayloadMode

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_trace_options(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TraceOptions:
        """Decode one TraceOptions."""
        return decode_trace_options(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_trace_options(self)

    @classmethod
    def from_json(cls, value: Json) -> TraceOptions:
        """Return one TraceOptions from one JSON value."""
        return from_json_trace_options(value)


def encode_trace_options(writer: BinaryWriter, value: TraceOptions) -> None:
    """Encode one TraceOptions."""
    encode_restore_mode(writer, value.restore)
    if value.path is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.path)
    if value.template is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.template)
    if value.chunk_size_mb is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_unsigned(value.chunk_size_mb)
    destack._generated.repository.config.runtime.execution.encode_replay_payload_mode(
        writer, value.payload
    )


def decode_trace_options(reader: BinaryReader) -> TraceOptions:
    """Decode one TraceOptions."""
    restore = decode_restore_mode(reader)
    path = reader.read_option(lambda: reader.read_string())
    template = reader.read_option(lambda: reader.read_string())
    chunk_size_mb = reader.read_option(lambda: reader.read_number())
    payload = destack._generated.repository.config.runtime.execution.decode_replay_payload_mode(
        reader
    )

    return TraceOptions(
        restore=restore,
        path=path,
        template=template,
        chunk_size_mb=chunk_size_mb,
        payload=payload,
    )


def to_json_trace_options(value: TraceOptions) -> Json:
    """Return one JSON value for one TraceOptions."""
    return {
        "restore": to_json_restore_mode(value.restore),
        **({} if value.path is None else {"path": value.path}),
        **({} if value.template is None else {"template": value.template}),
        **({} if value.chunk_size_mb is None else {"chunkSizeMb": value.chunk_size_mb}),
        "payload": destack._generated.repository.config.runtime.execution.to_json_replay_payload_mode(
            value.payload
        ),
    }


def from_json_trace_options(value: Json) -> TraceOptions:
    """Return one TraceOptions from one JSON value."""
    object_ = json_object(value)

    return TraceOptions(
        restore=from_json_restore_mode(json_field(object_, "restore")),
        path=json_optional(object_, "path", lambda value: json_string(value)),
        template=json_optional(object_, "template", lambda value: json_string(value)),
        chunk_size_mb=json_optional(
            object_, "chunkSizeMb", lambda value: json_int(value)
        ),
        payload=destack._generated.repository.config.runtime.execution.from_json_replay_payload_mode(
            json_field(object_, "payload")
        ),
    )


"""Restore contract for one world checkpoint or replay boundary."""
RestoreMode: typing.TypeAlias = typing.Literal["world"] | typing.Literal["image"]


def encode_restore_mode(writer: BinaryWriter, value: RestoreMode) -> None:
    """Encode one RestoreMode."""
    if value == "world":
        writer.write_unsigned(0)
    elif value == "image":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_restore_mode(reader: BinaryReader) -> RestoreMode:
    """Decode one RestoreMode."""
    variant = reader.read_number()

    if variant == 0:
        return "world"
    elif variant == 1:
        return "image"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_restore_mode(value: RestoreMode) -> Json:
    """Return one JSON value for one RestoreMode."""
    return value


def from_json_restore_mode(value: Json) -> RestoreMode:
    """Return one RestoreMode from one JSON value."""
    variant = json_string(value)

    if variant == "world":
        return "world"
    elif variant == "image":
        return "image"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


__all__ = [
    "TraceOptions",
    "encode_trace_options",
    "decode_trace_options",
    "to_json_trace_options",
    "from_json_trace_options",
    "RestoreMode",
    "encode_restore_mode",
    "decode_restore_mode",
    "to_json_restore_mode",
    "from_json_restore_mode",
]
