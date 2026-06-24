# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

"""Stable content identity for one interned string."""
StringId: typing.TypeAlias = int

def encode_string_id(writer: BinaryWriter, value: StringId) -> None: ...
def decode_string_id(reader: BinaryReader) -> StringId: ...
def to_json_string_id(value: StringId) -> Json: ...
def from_json_string_id(value: Json) -> StringId: ...

@dataclass(frozen=True, slots=True)
class StringPoolData:
    """Serialized string pool representation."""

    # stored strings sorted by stable id
    strings: Sequence[tuple[StringId, str]]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> StringPoolData: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> StringPoolData: ...

def encode_string_pool_data(writer: BinaryWriter, value: StringPoolData) -> None: ...
def decode_string_pool_data(reader: BinaryReader) -> StringPoolData: ...
def to_json_string_pool_data(value: StringPoolData) -> Json: ...
def from_json_string_pool_data(value: Json) -> StringPoolData: ...

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
