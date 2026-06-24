# generated bridge target, do not edit

from __future__ import annotations

import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json, json_string

"""A generic URI."""
Uri: typing.TypeAlias = str


def encode_uri(writer: BinaryWriter, value: Uri) -> None:
    """Encode one Uri."""
    writer.write_string(value)


def decode_uri(reader: BinaryReader) -> Uri:
    """Decode one Uri."""
    return reader.read_string()


def to_json_uri(value: Uri) -> Json:
    """Return one JSON value for one Uri."""
    return value


def from_json_uri(value: Json) -> Uri:
    """Return one Uri from one JSON value."""
    return json_string(value)


__all__ = [
    "Uri",
    "encode_uri",
    "decode_uri",
    "to_json_uri",
    "from_json_uri",
]
