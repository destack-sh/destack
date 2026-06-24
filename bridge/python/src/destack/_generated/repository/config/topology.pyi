# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

@dataclass(frozen=True, slots=True)
class Topology:
    """Package topology definition."""

    # declared topology entities
    entities: Mapping[str, TopologyEntity]
    # declared topology edges
    edges: Mapping[str, TopologyEdge]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Topology: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Topology: ...

def encode_topology(writer: BinaryWriter, value: Topology) -> None: ...
def decode_topology(reader: BinaryReader) -> Topology: ...
def to_json_topology(value: Topology) -> Json: ...
def from_json_topology(value: Json) -> Topology: ...

@dataclass(frozen=True, slots=True)
class TopologyEntity:
    """One declared topology entity."""

    # stable entity kind identifier
    kind: str
    # stable entity name
    name: str | None
    # entity labels
    labels: Mapping[str, str]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TopologyEntity: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TopologyEntity: ...

def encode_topology_entity(writer: BinaryWriter, value: TopologyEntity) -> None: ...
def decode_topology_entity(reader: BinaryReader) -> TopologyEntity: ...
def to_json_topology_entity(value: TopologyEntity) -> Json: ...
def from_json_topology_entity(value: Json) -> TopologyEntity: ...

@dataclass(frozen=True, slots=True)
class TopologyEdge:
    """One declared topology edge."""

    # stable edge kind identifier
    kind: str
    # stable edge name
    name: str | None
    # source node name
    from_: str
    # target node name
    to: str
    # edge labels
    labels: Mapping[str, str]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TopologyEdge: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TopologyEdge: ...

def encode_topology_edge(writer: BinaryWriter, value: TopologyEdge) -> None: ...
def decode_topology_edge(reader: BinaryReader) -> TopologyEdge: ...
def to_json_topology_edge(value: TopologyEdge) -> Json: ...
def from_json_topology_edge(value: Json) -> TopologyEdge: ...

__all__ = [
    "Topology",
    "encode_topology",
    "decode_topology",
    "to_json_topology",
    "from_json_topology",
    "TopologyEntity",
    "encode_topology_entity",
    "decode_topology_entity",
    "to_json_topology_entity",
    "from_json_topology_entity",
    "TopologyEdge",
    "encode_topology_edge",
    "decode_topology_edge",
    "to_json_topology_edge",
    "from_json_topology_edge",
]
