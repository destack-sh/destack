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
    json_object,
)


@dataclass(frozen=True, slots=True)
class Revision:
    """Content identity for one repository source and environment state."""

    value: builtins.bytes | bytearray | Sequence[int]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_revision(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Revision:
        """Decode one Revision."""
        return decode_revision(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_revision(self)

    @classmethod
    def from_json(cls, value: Json) -> Revision:
        """Return one Revision from one JSON value."""
        return from_json_revision(value)


def encode_revision(writer: BinaryWriter, value: Revision) -> None:
    """Encode one Revision."""
    writer.write_bytes(value.value)


def decode_revision(reader: BinaryReader) -> Revision:
    """Decode one Revision."""
    value_ = reader.read_bytes(32)

    return Revision(
        value=value_,
    )


def to_json_revision(value: Revision) -> Json:
    """Return one JSON value for one Revision."""
    return {
        "value": bytes_to_json(value.value),
    }


def from_json_revision(value: Json) -> Revision:
    """Return one Revision from one JSON value."""
    object_ = json_object(value)

    return Revision(
        value=bytes_from_json(json_field(object_, "value")),
    )


__all__ = [
    "Revision",
    "encode_revision",
    "decode_revision",
    "to_json_revision",
    "from_json_revision",
]
