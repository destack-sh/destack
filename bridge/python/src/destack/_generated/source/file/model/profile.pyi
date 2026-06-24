# generated bridge target, do not edit

from __future__ import annotations

import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

"""Unique identifier for profiles."""
ProfileId: typing.TypeAlias = int

def encode_profile_id(writer: BinaryWriter, value: ProfileId) -> None: ...
def decode_profile_id(reader: BinaryReader) -> ProfileId: ...
def to_json_profile_id(value: ProfileId) -> Json: ...
def from_json_profile_id(value: Json) -> ProfileId: ...

__all__ = [
    "ProfileId",
    "encode_profile_id",
    "decode_profile_id",
    "to_json_profile_id",
    "from_json_profile_id",
]
