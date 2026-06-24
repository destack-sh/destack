# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.dir.tree.origin

@dataclass(frozen=True, slots=True)
class SparseNodeMap:
    """Sorted sparse table keyed by node id."""

    # the sparse entries sorted by node id
    entries: Sequence[SparseNodeEntry]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> SparseNodeMap: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> SparseNodeMap: ...

def encode_sparse_node_map(writer: BinaryWriter, value: SparseNodeMap) -> None: ...
def decode_sparse_node_map(reader: BinaryReader) -> SparseNodeMap: ...
def to_json_sparse_node_map(value: SparseNodeMap) -> Json: ...
def from_json_sparse_node_map(value: Json) -> SparseNodeMap: ...

@dataclass(frozen=True, slots=True)
class SparseNodeEntry:
    """One sparse node table entry."""

    # the node id that owns the value
    node_id: int
    # the sparse value
    value: destack._generated.dir.tree.origin.Origin

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> SparseNodeEntry: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> SparseNodeEntry: ...

def encode_sparse_node_entry(writer: BinaryWriter, value: SparseNodeEntry) -> None: ...
def decode_sparse_node_entry(reader: BinaryReader) -> SparseNodeEntry: ...
def to_json_sparse_node_entry(value: SparseNodeEntry) -> Json: ...
def from_json_sparse_node_entry(value: Json) -> SparseNodeEntry: ...

__all__ = [
    "SparseNodeMap",
    "encode_sparse_node_map",
    "decode_sparse_node_map",
    "to_json_sparse_node_map",
    "from_json_sparse_node_map",
    "SparseNodeEntry",
    "encode_sparse_node_entry",
    "decode_sparse_node_entry",
    "to_json_sparse_node_entry",
    "from_json_sparse_node_entry",
]
