# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

@dataclass(frozen=True, slots=True)
class LocalNodeId:
    """Unique identifier for nodes in a local arena, parameterized by node type."""

    id: int

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> LocalNodeId: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> LocalNodeId: ...

def encode_local_node_id(writer: BinaryWriter, value: LocalNodeId) -> None: ...
def decode_local_node_id(reader: BinaryReader) -> LocalNodeId: ...
def to_json_local_node_id(value: LocalNodeId) -> Json: ...
def from_json_local_node_id(value: Json) -> LocalNodeId: ...

__all__ = [
    "LocalNodeId",
    "encode_local_node_id",
    "decode_local_node_id",
    "to_json_local_node_id",
    "from_json_local_node_id",
]
