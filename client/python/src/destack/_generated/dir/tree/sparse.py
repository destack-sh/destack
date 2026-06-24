# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_array,
    json_field,
    json_int,
    json_object,
)

import destack._generated.dir.tree.origin


@dataclass(frozen=True, slots=True)
class SparseNodeMap:
    """Sorted sparse table keyed by node id."""

    # the sparse entries sorted by node id
    entries: Sequence[SparseNodeEntry]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_sparse_node_map(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> SparseNodeMap:
        """Decode one SparseNodeMap."""
        return decode_sparse_node_map(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_sparse_node_map(self)

    @classmethod
    def from_json(cls, value: Json) -> SparseNodeMap:
        """Return one SparseNodeMap from one JSON value."""
        return from_json_sparse_node_map(value)


def encode_sparse_node_map(writer: BinaryWriter, value: SparseNodeMap) -> None:
    """Encode one SparseNodeMap."""
    writer.write_unsigned(len(value.entries))
    for item_value_entries_0 in value.entries:
        encode_sparse_node_entry(writer, item_value_entries_0)


def decode_sparse_node_map(reader: BinaryReader) -> SparseNodeMap:
    """Decode one SparseNodeMap."""
    entries = [decode_sparse_node_entry(reader) for _ in range(reader.read_number())]

    return SparseNodeMap(
        entries=entries,
    )


def to_json_sparse_node_map(value: SparseNodeMap) -> Json:
    """Return one JSON value for one SparseNodeMap."""
    return {
        "entries": [to_json_sparse_node_entry(item_0) for item_0 in value.entries],
    }


def from_json_sparse_node_map(value: Json) -> SparseNodeMap:
    """Return one SparseNodeMap from one JSON value."""
    object_ = json_object(value)

    return SparseNodeMap(
        entries=[
            from_json_sparse_node_entry(item_0)
            for item_0 in json_array(json_field(object_, "entries"))
        ],
    )


@dataclass(frozen=True, slots=True)
class SparseNodeEntry:
    """One sparse node table entry."""

    # the node id that owns the value
    node_id: int
    # the sparse value
    value: destack._generated.dir.tree.origin.Origin

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_sparse_node_entry(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> SparseNodeEntry:
        """Decode one SparseNodeEntry."""
        return decode_sparse_node_entry(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_sparse_node_entry(self)

    @classmethod
    def from_json(cls, value: Json) -> SparseNodeEntry:
        """Return one SparseNodeEntry from one JSON value."""
        return from_json_sparse_node_entry(value)


def encode_sparse_node_entry(writer: BinaryWriter, value: SparseNodeEntry) -> None:
    """Encode one SparseNodeEntry."""
    writer.write_unsigned(value.node_id)
    destack._generated.dir.tree.origin.encode_origin(writer, value.value)


def decode_sparse_node_entry(reader: BinaryReader) -> SparseNodeEntry:
    """Decode one SparseNodeEntry."""
    node_id = reader.read_number()
    value_ = destack._generated.dir.tree.origin.decode_origin(reader)

    return SparseNodeEntry(
        node_id=node_id,
        value=value_,
    )


def to_json_sparse_node_entry(value: SparseNodeEntry) -> Json:
    """Return one JSON value for one SparseNodeEntry."""
    return {
        "nodeId": value.node_id,
        "value": destack._generated.dir.tree.origin.to_json_origin(value.value),
    }


def from_json_sparse_node_entry(value: Json) -> SparseNodeEntry:
    """Return one SparseNodeEntry from one JSON value."""
    object_ = json_object(value)

    return SparseNodeEntry(
        node_id=json_int(json_field(object_, "nodeId")),
        value=destack._generated.dir.tree.origin.from_json_origin(
            json_field(object_, "value")
        ),
    )


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
