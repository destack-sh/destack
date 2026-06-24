# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_field,
    json_int,
    json_object,
)


@dataclass(frozen=True, slots=True)
class NodeIndexEntry:
    """Dense metadata for one global DIR node id."""

    # the packed local id and node type
    packed: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_node_index_entry(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> NodeIndexEntry:
        """Decode one NodeIndexEntry."""
        return decode_node_index_entry(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_node_index_entry(self)

    @classmethod
    def from_json(cls, value: Json) -> NodeIndexEntry:
        """Return one NodeIndexEntry from one JSON value."""
        return from_json_node_index_entry(value)


def encode_node_index_entry(writer: BinaryWriter, value: NodeIndexEntry) -> None:
    """Encode one NodeIndexEntry."""
    writer.write_unsigned(value.packed)


def decode_node_index_entry(reader: BinaryReader) -> NodeIndexEntry:
    """Decode one NodeIndexEntry."""
    packed = reader.read_number()

    return NodeIndexEntry(
        packed=packed,
    )


def to_json_node_index_entry(value: NodeIndexEntry) -> Json:
    """Return one JSON value for one NodeIndexEntry."""
    return {
        "packed": value.packed,
    }


def from_json_node_index_entry(value: Json) -> NodeIndexEntry:
    """Return one NodeIndexEntry from one JSON value."""
    object_ = json_object(value)

    return NodeIndexEntry(
        packed=json_int(json_field(object_, "packed")),
    )


__all__ = [
    "NodeIndexEntry",
    "encode_node_index_entry",
    "decode_node_index_entry",
    "to_json_node_index_entry",
    "from_json_node_index_entry",
]
