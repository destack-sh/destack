# generated client target, do not edit

from __future__ import annotations

import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json, json_int

"""Unique identifier for profiles."""
ProfileId: typing.TypeAlias = int


def encode_profile_id(writer: BinaryWriter, value: ProfileId) -> None:
    """Encode one ProfileId."""
    writer.write_unsigned(value)


def decode_profile_id(reader: BinaryReader) -> ProfileId:
    """Decode one ProfileId."""
    return reader.read_unsigned()


def to_json_profile_id(value: ProfileId) -> Json:
    """Return one JSON value for one ProfileId."""
    return value


def from_json_profile_id(value: Json) -> ProfileId:
    """Return one ProfileId from one JSON value."""
    return json_int(value)


__all__ = [
    "ProfileId",
    "encode_profile_id",
    "decode_profile_id",
    "to_json_profile_id",
    "from_json_profile_id",
]
