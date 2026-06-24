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
)

from destack._impl.mir.metadata.data import (
    DataLayoutImpl,
)


@dataclass(frozen=True, slots=True)
class DataLayout(DataLayoutImpl):
    """Target data layout for one MIR module."""

    # pointer size in bytes
    pointer_bytes: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_data_layout(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> DataLayout:
        """Decode one DataLayout."""
        return decode_data_layout(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_data_layout(self)

    @classmethod
    def from_json(cls, value: Json) -> DataLayout:
        """Return one DataLayout from one JSON value."""
        return from_json_data_layout(value)


def encode_data_layout(writer: BinaryWriter, value: DataLayout) -> None:
    """Encode one DataLayout."""
    writer.write_byte(value.pointer_bytes)


def decode_data_layout(reader: BinaryReader) -> DataLayout:
    """Decode one DataLayout."""
    pointer_bytes = reader.read_byte()

    return DataLayout(
        pointer_bytes=pointer_bytes,
    )


def to_json_data_layout(value: DataLayout) -> Json:
    """Return one JSON value for one DataLayout."""
    return {
        "pointerBytes": value.pointer_bytes,
    }


def from_json_data_layout(value: Json) -> DataLayout:
    """Return one DataLayout from one JSON value."""
    object_ = json_object(value)

    return DataLayout(
        pointer_bytes=json_int(json_field(object_, "pointerBytes")),
    )


__all__ = [
    "DataLayout",
    "encode_data_layout",
    "decode_data_layout",
    "to_json_data_layout",
    "from_json_data_layout",
]
