# generated client target, do not edit

from __future__ import annotations

import builtins

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    bytes_from_json,
    bytes_to_json,
    json_field,
    json_int,
    json_object,
)


@dataclass(frozen=True, slots=True)
class FrameHeader:
    """Frame header metadata."""

    # magic bytes for protocol frames
    magic: builtins.bytes | bytearray | Sequence[int]
    # frame format version
    version: int
    # reserved flags
    flags: int
    # payload length in bytes
    payload_len: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_frame_header(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> FrameHeader:
        """Decode one FrameHeader."""
        return decode_frame_header(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_frame_header(self)

    @classmethod
    def from_json(cls, value: Json) -> FrameHeader:
        """Return one FrameHeader from one JSON value."""
        return from_json_frame_header(value)


def encode_frame_header(writer: BinaryWriter, value: FrameHeader) -> None:
    """Encode one FrameHeader."""
    writer.write_bytes(value.magic)
    writer.write_unsigned(value.version)
    writer.write_unsigned(value.flags)
    writer.write_unsigned(value.payload_len)


def decode_frame_header(reader: BinaryReader) -> FrameHeader:
    """Decode one FrameHeader."""
    magic = reader.read_bytes(4)
    version = reader.read_number()
    flags = reader.read_number()
    payload_len = reader.read_number()

    return FrameHeader(
        magic=magic,
        version=version,
        flags=flags,
        payload_len=payload_len,
    )


def to_json_frame_header(value: FrameHeader) -> Json:
    """Return one JSON value for one FrameHeader."""
    return {
        "magic": bytes_to_json(value.magic),
        "version": value.version,
        "flags": value.flags,
        "payloadLen": value.payload_len,
    }


def from_json_frame_header(value: Json) -> FrameHeader:
    """Return one FrameHeader from one JSON value."""
    object_ = json_object(value)

    return FrameHeader(
        magic=bytes_from_json(json_field(object_, "magic")),
        version=json_int(json_field(object_, "version")),
        flags=json_int(json_field(object_, "flags")),
        payload_len=json_int(json_field(object_, "payloadLen")),
    )


__all__ = [
    "FrameHeader",
    "encode_frame_header",
    "decode_frame_header",
    "to_json_frame_header",
    "from_json_frame_header",
]
