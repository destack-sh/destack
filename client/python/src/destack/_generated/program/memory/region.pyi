# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.program.type

@dataclass(frozen=True, slots=True)
class StaticRegion:
    """One typed region inside static memory."""

    # the region id
    id: StaticId
    # the region byte offset
    offset: int
    # the region byte length
    byte_len: int
    # the region value type
    ty: destack._generated.program.type.TypeId
    # whether this region allows stores
    is_mutable: bool

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> StaticRegion: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> StaticRegion: ...

def encode_static_region(writer: BinaryWriter, value: StaticRegion) -> None: ...
def decode_static_region(reader: BinaryReader) -> StaticRegion: ...
def to_json_static_region(value: StaticRegion) -> Json: ...
def from_json_static_region(value: Json) -> StaticRegion: ...

"""One static region."""
StaticId: typing.TypeAlias = int

def encode_static_id(writer: BinaryWriter, value: StaticId) -> None: ...
def decode_static_id(reader: BinaryReader) -> StaticId: ...
def to_json_static_id(value: StaticId) -> Json: ...
def from_json_static_id(value: Json) -> StaticId: ...

__all__ = [
    "StaticRegion",
    "encode_static_region",
    "decode_static_region",
    "to_json_static_region",
    "from_json_static_region",
    "StaticId",
    "encode_static_id",
    "decode_static_id",
    "to_json_static_id",
    "from_json_static_id",
]
