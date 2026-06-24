# generated bridge target, do not edit

from __future__ import annotations

import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json, json_int

"""Identifier for one emitted profile counter, positional within a function."""
CounterId: typing.TypeAlias = int


def encode_counter_id(writer: BinaryWriter, value: CounterId) -> None:
    """Encode one CounterId."""
    writer.write_unsigned(value)


def decode_counter_id(reader: BinaryReader) -> CounterId:
    """Decode one CounterId."""
    return reader.read_number()


def to_json_counter_id(value: CounterId) -> Json:
    """Return one JSON value for one CounterId."""
    return value


def from_json_counter_id(value: Json) -> CounterId:
    """Return one CounterId from one JSON value."""
    return json_int(value)


__all__ = [
    "CounterId",
    "encode_counter_id",
    "decode_counter_id",
    "to_json_counter_id",
    "from_json_counter_id",
]
