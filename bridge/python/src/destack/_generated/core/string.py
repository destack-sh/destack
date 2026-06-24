# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_array,
    json_array_length,
    json_field,
    json_int,
    json_object,
    json_string,
)

"""Stable content identity for one interned string."""
StringId: typing.TypeAlias = int


def encode_string_id(writer: BinaryWriter, value: StringId) -> None:
    """Encode one StringId."""
    writer.write_unsigned(value)


def decode_string_id(reader: BinaryReader) -> StringId:
    """Decode one StringId."""
    return reader.read_unsigned()


def to_json_string_id(value: StringId) -> Json:
    """Return one JSON value for one StringId."""
    return value


def from_json_string_id(value: Json) -> StringId:
    """Return one StringId from one JSON value."""
    return json_int(value)


@dataclass(frozen=True, slots=True)
class StringPoolData:
    """Serialized string pool representation."""

    # stored strings sorted by stable id
    strings: Sequence[tuple[StringId, str]]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_string_pool_data(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> StringPoolData:
        """Decode one StringPoolData."""
        return decode_string_pool_data(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_string_pool_data(self)

    @classmethod
    def from_json(cls, value: Json) -> StringPoolData:
        """Return one StringPoolData from one JSON value."""
        return from_json_string_pool_data(value)


def encode_string_pool_data(writer: BinaryWriter, value: StringPoolData) -> None:
    """Encode one StringPoolData."""
    writer.write_unsigned(len(value.strings))
    for item_value_strings_0 in value.strings:
        encode_string_id(writer, item_value_strings_0[0])
        writer.write_string(item_value_strings_0[1])


def decode_string_pool_data(reader: BinaryReader) -> StringPoolData:
    """Decode one StringPoolData."""
    strings = [
        (
            decode_string_id(reader),
            reader.read_string(),
        )
        for _ in range(reader.read_number())
    ]

    return StringPoolData(
        strings=strings,
    )


def to_json_string_pool_data(value: StringPoolData) -> Json:
    """Return one JSON value for one StringPoolData."""
    return {
        "strings": [
            [to_json_string_id(item_0[0]), item_0[1]] for item_0 in value.strings
        ],
    }


def from_json_string_pool_data(value: Json) -> StringPoolData:
    """Return one StringPoolData from one JSON value."""
    object_ = json_object(value)

    return StringPoolData(
        strings=[
            (
                lambda items: (
                    from_json_string_id(items[0]),
                    json_string(items[1]),
                )
            )(json_array_length(item_0, 2))
            for item_0 in json_array(json_field(object_, "strings"))
        ],
    )


__all__ = [
    "StringId",
    "encode_string_id",
    "decode_string_id",
    "to_json_string_id",
    "from_json_string_id",
    "StringPoolData",
    "encode_string_pool_data",
    "decode_string_pool_data",
    "to_json_string_pool_data",
    "from_json_string_pool_data",
]
