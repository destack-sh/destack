# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

@dataclass(frozen=True, slots=True)
class NodeParentIndex:
    """The NodeParentIndex maps every node of one tree to its parent."""

    # first global node id this index covers; slots are keyed relative to it
    base: int
    parent_id_by_node_id: Sequence[int]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> NodeParentIndex: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> NodeParentIndex: ...

def encode_node_parent_index(writer: BinaryWriter, value: NodeParentIndex) -> None: ...
def decode_node_parent_index(reader: BinaryReader) -> NodeParentIndex: ...
def to_json_node_parent_index(value: NodeParentIndex) -> Json: ...
def from_json_node_parent_index(value: Json) -> NodeParentIndex: ...

__all__ = [
    "NodeParentIndex",
    "encode_node_parent_index",
    "decode_node_parent_index",
    "to_json_node_parent_index",
    "from_json_node_parent_index",
]
