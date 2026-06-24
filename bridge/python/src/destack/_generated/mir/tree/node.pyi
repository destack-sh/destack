# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

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

@dataclass(frozen=True, slots=True)
class LocalNodeIdAny:
    """Unique identifier for nodes with dynamic type in a local arena."""

    id: int
    ty: NodeType

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> LocalNodeIdAny: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> LocalNodeIdAny: ...

def encode_local_node_id_any(writer: BinaryWriter, value: LocalNodeIdAny) -> None: ...
def decode_local_node_id_any(reader: BinaryReader) -> LocalNodeIdAny: ...
def to_json_local_node_id_any(value: LocalNodeIdAny) -> Json: ...
def from_json_local_node_id_any(value: Json) -> LocalNodeIdAny: ...

"""The type of a MIR node."""
NodeType: typing.TypeAlias = (
    typing.Literal["function"]
    | typing.Literal["block"]
    | typing.Literal["instruction"]
    | typing.Literal["terminator"]
    | typing.Literal["local"]
    | typing.Literal["type"]
    | typing.Literal["typeAlias"]
    | typing.Literal["field"]
    | typing.Literal["global"]
)

def encode_node_type(writer: BinaryWriter, value: NodeType) -> None: ...
def decode_node_type(reader: BinaryReader) -> NodeType: ...
def to_json_node_type(value: NodeType) -> Json: ...
def from_json_node_type(value: Json) -> NodeType: ...

__all__ = [
    "LocalNodeId",
    "encode_local_node_id",
    "decode_local_node_id",
    "to_json_local_node_id",
    "from_json_local_node_id",
    "LocalNodeIdAny",
    "encode_local_node_id_any",
    "decode_local_node_id_any",
    "to_json_local_node_id_any",
    "from_json_local_node_id_any",
    "NodeType",
    "encode_node_type",
    "decode_node_type",
    "to_json_node_type",
    "from_json_node_type",
]
