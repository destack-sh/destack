# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_field,
    json_int,
    json_object,
    json_string,
)


@dataclass(frozen=True, slots=True)
class Abi:
    """Native ABI required by one native code payload."""

    # destack native ABI version
    version: int
    # target triple or equivalent target identity
    target: str
    # native pointer byte width expected by this code
    pointer_bytes: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_abi(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Abi:
        """Decode one Abi."""
        return decode_abi(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_abi(self)

    @classmethod
    def from_json(cls, value: Json) -> Abi:
        """Return one Abi from one JSON value."""
        return from_json_abi(value)


def encode_abi(writer: BinaryWriter, value: Abi) -> None:
    """Encode one Abi."""
    writer.write_unsigned(value.version)
    writer.write_string(value.target)
    writer.write_byte(value.pointer_bytes)


def decode_abi(reader: BinaryReader) -> Abi:
    """Decode one Abi."""
    version = reader.read_number()
    target = reader.read_string()
    pointer_bytes = reader.read_byte()

    return Abi(
        version=version,
        target=target,
        pointer_bytes=pointer_bytes,
    )


def to_json_abi(value: Abi) -> Json:
    """Return one JSON value for one Abi."""
    return {
        "version": value.version,
        "target": value.target,
        "pointerBytes": value.pointer_bytes,
    }


def from_json_abi(value: Json) -> Abi:
    """Return one Abi from one JSON value."""
    object_ = json_object(value)

    return Abi(
        version=json_int(json_field(object_, "version")),
        target=json_string(json_field(object_, "target")),
        pointer_bytes=json_int(json_field(object_, "pointerBytes")),
    )


__all__ = [
    "Abi",
    "encode_abi",
    "decode_abi",
    "to_json_abi",
    "from_json_abi",
]
