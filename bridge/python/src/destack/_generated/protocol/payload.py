# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes


@dataclass(frozen=True, slots=True)
class BinaryPayload:
    """Binary payload wrapper for protocol message bodies."""

    """Payload body."""
    body: PayloadBody


def encode_binary_payload(writer: Writer, value: BinaryPayload) -> None:
    encode_payload_body(writer, value.body)


def decode_binary_payload(reader: Reader) -> BinaryPayload:
    field_0 = decode_payload_body(reader)

    return BinaryPayload(
        body=field_0,
    )


@dataclass(frozen=True, slots=True)
class PayloadBodyInline:
    """Inline bytes."""

    bytes: bytes | bytearray | Sequence[int]
    kind: Literal["inline"] = "inline"


@dataclass(frozen=True, slots=True)
class PayloadBodyDeferred:
    """Deferred payload identified by id."""

    id: PayloadId
    total_bytes: int
    kind: Literal["deferred"] = "deferred"


"""Payload body representation."""
PayloadBody: TypeAlias = PayloadBodyInline | PayloadBodyDeferred


def encode_payload_body(writer: Writer, value: PayloadBody) -> None:
    if value.kind == "inline":
        writer.write_unsigned(0)
        writer.write_byte_slice(value.bytes)
    elif value.kind == "deferred":
        writer.write_unsigned(1)
        encode_payload_id(writer, value.id)
        writer.write_unsigned(value.total_bytes)
    else:
        raise SerdeError("unknown enum variant")


def decode_payload_body(reader: Reader) -> PayloadBody:
    variant = reader.read_number()

    if variant == 0:
        field_0 = reader.read_byte_slice()

        return PayloadBodyInline(
            bytes=field_0,
        )
    elif variant == 1:
        field_0 = decode_payload_id(reader)
        field_1 = reader.read_number()

        return PayloadBodyDeferred(
            id=field_0,
            total_bytes=field_1,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


@dataclass(frozen=True, slots=True)
class PayloadId:
    """Unique identifier for payload transfers."""

    field_0: int


def encode_payload_id(writer: Writer, value: PayloadId) -> None:
    writer.write_unsigned(value.field_0)


def decode_payload_id(reader: Reader) -> PayloadId:
    field_0 = reader.read_number()

    return PayloadId(
        field_0=field_0,
    )


@dataclass(frozen=True, slots=True)
class PayloadChunkNotification:
    """Notification for chunked payload data."""

    """Payload id for the chunk stream."""
    id: PayloadId
    """Zero based chunk index."""
    index: int
    """Total chunks expected."""
    total: int
    """Chunk bytes."""
    bytes: bytes | bytearray | Sequence[int]
    """Whether this chunk is the final chunk."""
    done: bool


def encode_payload_chunk_notification(
    writer: Writer, value: PayloadChunkNotification
) -> None:
    encode_payload_id(writer, value.id)
    writer.write_unsigned(value.index)
    writer.write_unsigned(value.total)
    writer.write_byte_slice(value.bytes)
    writer.write_bool(value.done)


def decode_payload_chunk_notification(reader: Reader) -> PayloadChunkNotification:
    field_0 = decode_payload_id(reader)
    field_1 = reader.read_number()
    field_2 = reader.read_number()
    field_3 = reader.read_byte_slice()
    field_4 = reader.read_bool()

    return PayloadChunkNotification(
        id=field_0,
        index=field_1,
        total=field_2,
        bytes=field_3,
        done=field_4,
    )


__all__ = [
    "BinaryPayload",
    "encode_binary_payload",
    "decode_binary_payload",
    "PayloadBody",
    "encode_payload_body",
    "decode_payload_body",
    "PayloadBodyInline",
    "PayloadBodyDeferred",
    "PayloadId",
    "encode_payload_id",
    "decode_payload_id",
    "PayloadChunkNotification",
    "encode_payload_chunk_notification",
    "decode_payload_chunk_notification",
]
