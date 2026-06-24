from __future__ import annotations

from collections.abc import Sequence

from destack._generated.protocol.envelope import (
    ProtocolMessage,
    decode_protocol_message,
    encode_protocol_message,
)
from destack.protocol.serde import decode_value, encode_value

FRAME_MAGIC = bytes([0x44, 0x53, 0x57, 0x50])
FRAME_VERSION = 1
FRAME_HEADER_SIZE = 12


class ProtocolCodecError(Exception):
    """Error thrown while encoding or decoding workspace protocol frames."""


def encode_message(message: ProtocolMessage) -> bytes:
    """Encode one protocol message payload."""

    return encode_value(lambda writer: encode_protocol_message(writer, message))


def decode_message(payload: bytes | bytearray | Sequence[int]) -> ProtocolMessage:
    """Decode one protocol message payload."""

    return decode_value(payload, decode_protocol_message)


def encode_frame(message: ProtocolMessage) -> bytes:
    """Encode one protocol message frame."""

    payload = encode_message(message)

    return encode_payload_frame(payload)


def decode_frame(frame: bytes | bytearray | Sequence[int]) -> ProtocolMessage:
    """Decode one protocol message frame."""

    payload = decode_payload_frame(frame)

    return decode_message(payload)


def encode_payload_frame(payload: bytes | bytearray | Sequence[int]) -> bytes:
    """Encode one protocol payload frame."""

    payload = bytes(payload)
    header = encode_header(len(payload))

    return header + payload


def decode_payload_frame(frame: bytes | bytearray | Sequence[int]) -> bytes:
    """Decode one protocol payload frame."""

    frame = bytes(frame)
    payload_len = decode_header(frame)
    end = FRAME_HEADER_SIZE + payload_len
    if len(frame) < end:
        raise ProtocolCodecError("unexpected end of frame payload")

    return frame[FRAME_HEADER_SIZE:end]


def encode_header(payload_len: int) -> bytes:
    """Encode one frame header."""

    if not isinstance(payload_len, int) or payload_len < 0 or payload_len > 0xFFFFFFFF:
        raise ProtocolCodecError(f"invalid payload length: {payload_len}")

    return (
        FRAME_MAGIC
        + FRAME_VERSION.to_bytes(2, "little")
        + (0).to_bytes(2, "little")
        + payload_len.to_bytes(4, "little")
    )


def decode_header(frame: bytes) -> int:
    """Decode one frame header and return its payload length."""

    if len(frame) < FRAME_HEADER_SIZE:
        raise ProtocolCodecError("unexpected end of frame header")

    magic = frame[:4]
    version = int.from_bytes(frame[4:6], "little")
    payload_len = int.from_bytes(frame[8:12], "little")
    if magic != FRAME_MAGIC:
        raise ProtocolCodecError("invalid frame magic")
    if version != FRAME_VERSION:
        raise ProtocolCodecError(f"unsupported frame version: {version}")

    return payload_len
