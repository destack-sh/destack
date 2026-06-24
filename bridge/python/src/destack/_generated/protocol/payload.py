# generated bridge target, do not edit

from __future__ import annotations

import builtins

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    bytes_from_json,
    bytes_to_json,
    json_bool,
    json_field,
    json_int,
    json_object,
    json_string,
)


@dataclass(frozen=True, slots=True)
class BinaryPayload:
    """Binary payload wrapper for protocol message bodies."""

    # payload body
    body: PayloadBody

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_binary_payload(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> BinaryPayload:
        """Decode one BinaryPayload."""
        return decode_binary_payload(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_binary_payload(self)

    @classmethod
    def from_json(cls, value: Json) -> BinaryPayload:
        """Return one BinaryPayload from one JSON value."""
        return from_json_binary_payload(value)


def encode_binary_payload(writer: BinaryWriter, value: BinaryPayload) -> None:
    """Encode one BinaryPayload."""
    encode_payload_body(writer, value.body)


def decode_binary_payload(reader: BinaryReader) -> BinaryPayload:
    """Decode one BinaryPayload."""
    body = decode_payload_body(reader)

    return BinaryPayload(
        body=body,
    )


def to_json_binary_payload(value: BinaryPayload) -> Json:
    """Return one JSON value for one BinaryPayload."""
    return {
        "body": to_json_payload_body(value.body),
    }


def from_json_binary_payload(value: Json) -> BinaryPayload:
    """Return one BinaryPayload from one JSON value."""
    object_ = json_object(value)

    return BinaryPayload(
        body=from_json_payload_body(json_field(object_, "body")),
    )


@dataclass(frozen=True, slots=True)
class PayloadBodyInline:
    """Inline bytes."""

    bytes: builtins.bytes | bytearray | Sequence[int]
    kind: typing.Literal["inline"] = "inline"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_payload_body(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_payload_body(self)


@dataclass(frozen=True, slots=True)
class PayloadBodyDeferred:
    """Deferred payload identified by id."""

    id: PayloadId
    total_bytes: int
    kind: typing.Literal["deferred"] = "deferred"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_payload_body(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_payload_body(self)


"""Payload body representation."""
PayloadBody: typing.TypeAlias = PayloadBodyInline | PayloadBodyDeferred


def encode_payload_body(writer: BinaryWriter, value: PayloadBody) -> None:
    """Encode one PayloadBody."""
    if value.kind == "inline":
        writer.write_unsigned(0)
        writer.write_byte_slice(value.bytes)
    elif value.kind == "deferred":
        writer.write_unsigned(1)
        encode_payload_id(writer, value.id)
        writer.write_unsigned(value.total_bytes)
    else:
        raise SerdeError("unknown enum variant")


def decode_payload_body(reader: BinaryReader) -> PayloadBody:
    """Decode one PayloadBody."""
    variant = reader.read_number()

    if variant == 0:
        bytes = reader.read_byte_slice()

        return PayloadBodyInline(
            bytes=bytes,
        )
    elif variant == 1:
        id = decode_payload_id(reader)
        total_bytes = reader.read_number()

        return PayloadBodyDeferred(
            id=id,
            total_bytes=total_bytes,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_payload_body(value: PayloadBody) -> Json:
    """Return one JSON value for one PayloadBody."""
    if value.kind == "inline":
        return {
            "kind": "inline",
            "bytes": bytes_to_json(value.bytes),
        }
    elif value.kind == "deferred":
        return {
            "kind": "deferred",
            "id": to_json_payload_id(value.id),
            "totalBytes": value.total_bytes,
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_payload_body(value: Json) -> PayloadBody:
    """Return one PayloadBody from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "inline":
        return PayloadBodyInline(
            bytes=bytes_from_json(json_field(object_, "bytes")),
        )
    elif kind == "deferred":
        return PayloadBodyDeferred(
            id=from_json_payload_id(json_field(object_, "id")),
            total_bytes=json_int(json_field(object_, "totalBytes")),
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


"""Unique identifier for payload transfers."""
PayloadId: typing.TypeAlias = int


def encode_payload_id(writer: BinaryWriter, value: PayloadId) -> None:
    """Encode one PayloadId."""
    writer.write_unsigned(value)


def decode_payload_id(reader: BinaryReader) -> PayloadId:
    """Decode one PayloadId."""
    return reader.read_number()


def to_json_payload_id(value: PayloadId) -> Json:
    """Return one JSON value for one PayloadId."""
    return value


def from_json_payload_id(value: Json) -> PayloadId:
    """Return one PayloadId from one JSON value."""
    return json_int(value)


@dataclass(frozen=True, slots=True)
class PayloadChunkNotification:
    """Notification for chunked payload data."""

    # payload id for the chunk stream
    id: PayloadId
    # zero based chunk index
    index: int
    # total chunks expected
    total: int
    # chunk bytes
    bytes: builtins.bytes | bytearray | Sequence[int]
    # whether this chunk is the final chunk
    done: bool

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_payload_chunk_notification(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> PayloadChunkNotification:
        """Decode one PayloadChunkNotification."""
        return decode_payload_chunk_notification(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_payload_chunk_notification(self)

    @classmethod
    def from_json(cls, value: Json) -> PayloadChunkNotification:
        """Return one PayloadChunkNotification from one JSON value."""
        return from_json_payload_chunk_notification(value)


def encode_payload_chunk_notification(
    writer: BinaryWriter, value: PayloadChunkNotification
) -> None:
    """Encode one PayloadChunkNotification."""
    encode_payload_id(writer, value.id)
    writer.write_unsigned(value.index)
    writer.write_unsigned(value.total)
    writer.write_byte_slice(value.bytes)
    writer.write_bool(value.done)


def decode_payload_chunk_notification(reader: BinaryReader) -> PayloadChunkNotification:
    """Decode one PayloadChunkNotification."""
    id = decode_payload_id(reader)
    index = reader.read_number()
    total = reader.read_number()
    bytes = reader.read_byte_slice()
    done = reader.read_bool()

    return PayloadChunkNotification(
        id=id,
        index=index,
        total=total,
        bytes=bytes,
        done=done,
    )


def to_json_payload_chunk_notification(value: PayloadChunkNotification) -> Json:
    """Return one JSON value for one PayloadChunkNotification."""
    return {
        "id": to_json_payload_id(value.id),
        "index": value.index,
        "total": value.total,
        "bytes": bytes_to_json(value.bytes),
        "done": value.done,
    }


def from_json_payload_chunk_notification(value: Json) -> PayloadChunkNotification:
    """Return one PayloadChunkNotification from one JSON value."""
    object_ = json_object(value)

    return PayloadChunkNotification(
        id=from_json_payload_id(json_field(object_, "id")),
        index=json_int(json_field(object_, "index")),
        total=json_int(json_field(object_, "total")),
        bytes=bytes_from_json(json_field(object_, "bytes")),
        done=json_bool(json_field(object_, "done")),
    )


__all__ = [
    "BinaryPayload",
    "encode_binary_payload",
    "decode_binary_payload",
    "to_json_binary_payload",
    "from_json_binary_payload",
    "PayloadBody",
    "encode_payload_body",
    "decode_payload_body",
    "to_json_payload_body",
    "from_json_payload_body",
    "PayloadBodyInline",
    "PayloadBodyDeferred",
    "PayloadId",
    "encode_payload_id",
    "decode_payload_id",
    "to_json_payload_id",
    "from_json_payload_id",
    "PayloadChunkNotification",
    "encode_payload_chunk_notification",
    "decode_payload_chunk_notification",
    "to_json_payload_chunk_notification",
    "from_json_payload_chunk_notification",
]
