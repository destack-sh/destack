# generated bridge target, do not edit

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


@dataclass(frozen=True, slots=True)
class NodeParentIndex:
    """The NodeParentIndex maps every node of one tree to its parent."""

    # first global node id this index covers; slots are keyed relative to it
    base: int
    parent_id_by_node_id: Sequence[int]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_node_parent_index(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> NodeParentIndex:
        """Decode one NodeParentIndex."""
        return decode_node_parent_index(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_node_parent_index(self)

    @classmethod
    def from_json(cls, value: Json) -> NodeParentIndex:
        """Return one NodeParentIndex from one JSON value."""
        return from_json_node_parent_index(value)


def encode_node_parent_index(writer: BinaryWriter, value: NodeParentIndex) -> None:
    """Encode one NodeParentIndex."""
    writer.write_unsigned(value.base)
    writer.write_unsigned(len(value.parent_id_by_node_id))
    for item_value_parent_id_by_node_id_0 in value.parent_id_by_node_id:
        writer.write_unsigned(item_value_parent_id_by_node_id_0)


def decode_node_parent_index(reader: BinaryReader) -> NodeParentIndex:
    """Decode one NodeParentIndex."""
    base = reader.read_number()
    parent_id_by_node_id = [reader.read_number() for _ in range(reader.read_number())]

    return NodeParentIndex(
        base=base,
        parent_id_by_node_id=parent_id_by_node_id,
    )


def to_json_node_parent_index(value: NodeParentIndex) -> Json:
    """Return one JSON value for one NodeParentIndex."""
    return {
        "base": value.base,
        "parentIdByNodeId": [item_0 for item_0 in value.parent_id_by_node_id],
    }


def from_json_node_parent_index(value: Json) -> NodeParentIndex:
    """Return one NodeParentIndex from one JSON value."""
    object_ = json_object(value)

    return NodeParentIndex(
        base=json_int(json_field(object_, "base")),
        parent_id_by_node_id=[
            json_int(item_0)
            for item_0 in json_array(json_field(object_, "parentIdByNodeId"))
        ],
    )


__all__ = [
    "NodeParentIndex",
    "encode_node_parent_index",
    "decode_node_parent_index",
    "to_json_node_parent_index",
    "from_json_node_parent_index",
]
