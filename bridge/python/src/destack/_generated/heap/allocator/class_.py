# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_array,
    json_field,
    json_int,
    json_object,
)


@dataclass(frozen=True, slots=True)
class SizeClassTable:
    """One canonical size-class table used by the heap."""

    # the ordered size classes in bytes
    classes: Sequence[SizeClass]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_size_class_table(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> SizeClassTable:
        """Decode one SizeClassTable."""
        return decode_size_class_table(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_size_class_table(self)

    @classmethod
    def from_json(cls, value: Json) -> SizeClassTable:
        """Return one SizeClassTable from one JSON value."""
        return from_json_size_class_table(value)


def encode_size_class_table(writer: BinaryWriter, value: SizeClassTable) -> None:
    """Encode one SizeClassTable."""
    writer.write_unsigned(len(value.classes))
    for item_value_classes_0 in value.classes:
        encode_size_class(writer, item_value_classes_0)


def decode_size_class_table(reader: BinaryReader) -> SizeClassTable:
    """Decode one SizeClassTable."""
    classes = [decode_size_class(reader) for _ in range(reader.read_number())]

    return SizeClassTable(
        classes=classes,
    )


def to_json_size_class_table(value: SizeClassTable) -> Json:
    """Return one JSON value for one SizeClassTable."""
    return {
        "classes": [to_json_size_class(item_0) for item_0 in value.classes],
    }


def from_json_size_class_table(value: Json) -> SizeClassTable:
    """Return one SizeClassTable from one JSON value."""
    object_ = json_object(value)

    return SizeClassTable(
        classes=[
            from_json_size_class(item_0)
            for item_0 in json_array(json_field(object_, "classes"))
        ],
    )


@dataclass(frozen=True, slots=True)
class SizeClass:
    """One fixed-size small block class."""

    # the slot payload size in bytes
    bytes: int
    # the span width in bytes, or zero for the default span size
    span_size_bytes: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_size_class(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> SizeClass:
        """Decode one SizeClass."""
        return decode_size_class(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_size_class(self)

    @classmethod
    def from_json(cls, value: Json) -> SizeClass:
        """Return one SizeClass from one JSON value."""
        return from_json_size_class(value)


def encode_size_class(writer: BinaryWriter, value: SizeClass) -> None:
    """Encode one SizeClass."""
    writer.write_unsigned(value.bytes)
    writer.write_unsigned(value.span_size_bytes)


def decode_size_class(reader: BinaryReader) -> SizeClass:
    """Decode one SizeClass."""
    bytes = reader.read_number()
    span_size_bytes = reader.read_number()

    return SizeClass(
        bytes=bytes,
        span_size_bytes=span_size_bytes,
    )


def to_json_size_class(value: SizeClass) -> Json:
    """Return one JSON value for one SizeClass."""
    return {
        "bytes": value.bytes,
        "spanSizeBytes": value.span_size_bytes,
    }


def from_json_size_class(value: Json) -> SizeClass:
    """Return one SizeClass from one JSON value."""
    object_ = json_object(value)

    return SizeClass(
        bytes=json_int(json_field(object_, "bytes")),
        span_size_bytes=json_int(json_field(object_, "spanSizeBytes")),
    )


__all__ = [
    "SizeClassTable",
    "encode_size_class_table",
    "decode_size_class_table",
    "to_json_size_class_table",
    "from_json_size_class_table",
    "SizeClass",
    "encode_size_class",
    "decode_size_class",
    "to_json_size_class",
    "from_json_size_class",
]
