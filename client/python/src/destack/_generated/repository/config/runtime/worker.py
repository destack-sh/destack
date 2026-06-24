# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_field,
    json_int,
    json_object,
    json_optional,
)


@dataclass(frozen=True, slots=True)
class WorkerOptions:
    """Runtime worker configuration."""

    # maximum number of live workers in one runtime
    limit: int | None
    # maximum number of host threads allocated to workers
    thread_limit: int | None
    # stack reservation in bytes for one worker
    stack_bytes: int | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_worker_options(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> WorkerOptions:
        """Decode one WorkerOptions."""
        return decode_worker_options(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_worker_options(self)

    @classmethod
    def from_json(cls, value: Json) -> WorkerOptions:
        """Return one WorkerOptions from one JSON value."""
        return from_json_worker_options(value)


def encode_worker_options(writer: BinaryWriter, value: WorkerOptions) -> None:
    """Encode one WorkerOptions."""
    if value.limit is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_unsigned(value.limit)
    if value.thread_limit is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_unsigned(value.thread_limit)
    if value.stack_bytes is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_unsigned(value.stack_bytes)


def decode_worker_options(reader: BinaryReader) -> WorkerOptions:
    """Decode one WorkerOptions."""
    limit = reader.read_option(lambda: reader.read_number())
    thread_limit = reader.read_option(lambda: reader.read_number())
    stack_bytes = reader.read_option(lambda: reader.read_number())

    return WorkerOptions(
        limit=limit,
        thread_limit=thread_limit,
        stack_bytes=stack_bytes,
    )


def to_json_worker_options(value: WorkerOptions) -> Json:
    """Return one JSON value for one WorkerOptions."""
    return {
        **({} if value.limit is None else {"limit": value.limit}),
        **({} if value.thread_limit is None else {"threadLimit": value.thread_limit}),
        **({} if value.stack_bytes is None else {"stackBytes": value.stack_bytes}),
    }


def from_json_worker_options(value: Json) -> WorkerOptions:
    """Return one WorkerOptions from one JSON value."""
    object_ = json_object(value)

    return WorkerOptions(
        limit=json_optional(object_, "limit", lambda value: json_int(value)),
        thread_limit=json_optional(
            object_, "threadLimit", lambda value: json_int(value)
        ),
        stack_bytes=json_optional(object_, "stackBytes", lambda value: json_int(value)),
    )


__all__ = [
    "WorkerOptions",
    "encode_worker_options",
    "decode_worker_options",
    "to_json_worker_options",
    "from_json_worker_options",
]
