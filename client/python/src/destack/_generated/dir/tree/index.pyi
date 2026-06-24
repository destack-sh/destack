# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

@dataclass(frozen=True, slots=True)
class NodeIndexEntry:
    """Dense metadata for one global DIR node id."""

    # the packed local id and node type
    packed: int

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> NodeIndexEntry: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> NodeIndexEntry: ...

def encode_node_index_entry(writer: BinaryWriter, value: NodeIndexEntry) -> None: ...
def decode_node_index_entry(reader: BinaryReader) -> NodeIndexEntry: ...
def to_json_node_index_entry(value: NodeIndexEntry) -> Json: ...
def from_json_node_index_entry(value: Json) -> NodeIndexEntry: ...

__all__ = [
    "NodeIndexEntry",
    "encode_node_index_entry",
    "decode_node_index_entry",
    "to_json_node_index_entry",
    "from_json_node_index_entry",
]
