# generated client target, do not edit

from __future__ import annotations

import builtins

from collections.abc import Mapping, Sequence
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.program.memory.region

@dataclass(frozen=True, slots=True)
class StaticSpace:
    """Static memory."""

    # the static bytes
    bytes: builtins.bytes | bytearray | Sequence[int]
    # static regions
    regions: Sequence[destack._generated.program.memory.region.StaticRegion]
    # region index by id
    region_by_id: Mapping[destack._generated.program.memory.region.StaticId, int]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> StaticSpace: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> StaticSpace: ...

def encode_static_space(writer: BinaryWriter, value: StaticSpace) -> None: ...
def decode_static_space(reader: BinaryReader) -> StaticSpace: ...
def to_json_static_space(value: StaticSpace) -> Json: ...
def from_json_static_space(value: Json) -> StaticSpace: ...

__all__ = [
    "StaticSpace",
    "encode_static_space",
    "decode_static_space",
    "to_json_static_space",
    "from_json_static_space",
]
