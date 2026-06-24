# generated bridge target, do not edit

from __future__ import annotations

import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

"""Unique identifier for one source package."""
PackageId: typing.TypeAlias = int

def encode_package_id(writer: BinaryWriter, value: PackageId) -> None: ...
def decode_package_id(reader: BinaryReader) -> PackageId: ...
def to_json_package_id(value: PackageId) -> Json: ...
def from_json_package_id(value: Json) -> PackageId: ...

__all__ = [
    "PackageId",
    "encode_package_id",
    "decode_package_id",
    "to_json_package_id",
    "from_json_package_id",
]
