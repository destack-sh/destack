# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.core.string

@dataclass(frozen=True, slots=True)
class OriginTable:
    """Origin records for derived nodes: dense per-node slots into a packed arena."""

    # the arena slot for each node index, present for derived nodes
    slot_by_index: Sequence[int | None]
    # the packed origin records, emptied when an origin moves to another node
    origins: Sequence[Origin | None]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> OriginTable: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> OriginTable: ...

def encode_origin_table(writer: BinaryWriter, value: OriginTable) -> None: ...
def decode_origin_table(reader: BinaryReader) -> OriginTable: ...
def to_json_origin_table(value: OriginTable) -> Json: ...
def from_json_origin_table(value: Json) -> OriginTable: ...

@dataclass(frozen=True, slots=True)
class Origin:
    """How one derived tree node came to be."""

    # the transform that created the node, a dotted name like `optimize.inline`
    derivation: destack._generated.core.string.StringId
    # the same-tree nodes the node derives from, interpretation per derivation
    parents: Sequence[int]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Origin: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Origin: ...

def encode_origin(writer: BinaryWriter, value: Origin) -> None: ...
def decode_origin(reader: BinaryReader) -> Origin: ...
def to_json_origin(value: Origin) -> Json: ...
def from_json_origin(value: Json) -> Origin: ...

__all__ = [
    "OriginTable",
    "encode_origin_table",
    "decode_origin_table",
    "to_json_origin_table",
    "from_json_origin_table",
    "Origin",
    "encode_origin",
    "decode_origin",
    "to_json_origin",
    "from_json_origin",
]
