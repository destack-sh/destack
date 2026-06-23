# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes


@dataclass(frozen=True, slots=True)
class FrameHeader:
    """Frame header metadata."""

    """Magic bytes for protocol frames."""
    magic: bytes | bytearray | Sequence[int]
    """Frame format version."""
    version: int
    """Reserved flags."""
    flags: int
    """Payload length in bytes."""
    payload_len: int


def encode_frame_header(writer: Writer, value: FrameHeader) -> None:
    writer.write_bytes(value.magic)
    writer.write_unsigned(value.version)
    writer.write_unsigned(value.flags)
    writer.write_unsigned(value.payload_len)


def decode_frame_header(reader: Reader) -> FrameHeader:
    field_0 = reader.read_bytes(4)
    field_1 = reader.read_number()
    field_2 = reader.read_number()
    field_3 = reader.read_number()

    return FrameHeader(
        magic=field_0,
        version=field_1,
        flags=field_2,
        payload_len=field_3,
    )


__all__ = [
    "FrameHeader",
    "encode_frame_header",
    "decode_frame_header",
]
