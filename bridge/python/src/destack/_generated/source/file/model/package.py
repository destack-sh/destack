# generated bridge target, do not edit

from __future__ import annotations

import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json, json_int

"""Unique identifier for one source package."""
PackageId: typing.TypeAlias = int


def encode_package_id(writer: BinaryWriter, value: PackageId) -> None:
    """Encode one PackageId."""
    writer.write_unsigned(value)


def decode_package_id(reader: BinaryReader) -> PackageId:
    """Decode one PackageId."""
    return reader.read_unsigned()


def to_json_package_id(value: PackageId) -> Json:
    """Return one JSON value for one PackageId."""
    return value


def from_json_package_id(value: Json) -> PackageId:
    """Return one PackageId from one JSON value."""
    return json_int(value)


__all__ = [
    "PackageId",
    "encode_package_id",
    "decode_package_id",
    "to_json_package_id",
    "from_json_package_id",
]
