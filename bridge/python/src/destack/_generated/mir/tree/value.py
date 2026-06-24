# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_field,
    json_int,
    json_object,
)

from destack._impl.mir.tree.value import (
    ValueSliceImpl,
)

"""SSA value (virtual register)."""
Value: typing.TypeAlias = int


def encode_value(writer: BinaryWriter, value: Value) -> None:
    """Encode one Value."""
    writer.write_unsigned(value)


def decode_value(reader: BinaryReader) -> Value:
    """Decode one Value."""
    return reader.read_number()


def to_json_value(value: Value) -> Json:
    """Return one JSON value for one Value."""
    return value


def from_json_value(value: Json) -> Value:
    """Return one Value from one JSON value."""
    return json_int(value)


@dataclass(frozen=True, slots=True)
class ValueSlice(ValueSliceImpl):
    """Compact reference to a value list stored in the MIR tree."""

    # start index in the value buffer
    start: int
    # number of values in the slice
    count: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_value_slice(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ValueSlice:
        """Decode one ValueSlice."""
        return decode_value_slice(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_value_slice(self)

    @classmethod
    def from_json(cls, value: Json) -> ValueSlice:
        """Return one ValueSlice from one JSON value."""
        return from_json_value_slice(value)


def encode_value_slice(writer: BinaryWriter, value: ValueSlice) -> None:
    """Encode one ValueSlice."""
    writer.write_unsigned(value.start)
    writer.write_unsigned(value.count)


def decode_value_slice(reader: BinaryReader) -> ValueSlice:
    """Decode one ValueSlice."""
    start = reader.read_number()
    count = reader.read_number()

    return ValueSlice(
        start=start,
        count=count,
    )


def to_json_value_slice(value: ValueSlice) -> Json:
    """Return one JSON value for one ValueSlice."""
    return {
        "start": value.start,
        "count": value.count,
    }


def from_json_value_slice(value: Json) -> ValueSlice:
    """Return one ValueSlice from one JSON value."""
    object_ = json_object(value)

    return ValueSlice(
        start=json_int(json_field(object_, "start")),
        count=json_int(json_field(object_, "count")),
    )


__all__ = [
    "Value",
    "encode_value",
    "decode_value",
    "to_json_value",
    "from_json_value",
    "ValueSlice",
    "encode_value_slice",
    "decode_value_slice",
    "to_json_value_slice",
    "from_json_value_slice",
]
