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
class LocalNodeId:
    """Unique identifier for nodes in a local arena, parameterized by node type."""

    id: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_local_node_id(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> LocalNodeId:
        """Decode one LocalNodeId."""
        return decode_local_node_id(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_local_node_id(self)

    @classmethod
    def from_json(cls, value: Json) -> LocalNodeId:
        """Return one LocalNodeId from one JSON value."""
        return from_json_local_node_id(value)


def encode_local_node_id(writer: BinaryWriter, value: LocalNodeId) -> None:
    """Encode one LocalNodeId."""
    writer.write_unsigned(value.id)


def decode_local_node_id(reader: BinaryReader) -> LocalNodeId:
    """Decode one LocalNodeId."""
    id = reader.read_number()

    return LocalNodeId(
        id=id,
    )


def to_json_local_node_id(value: LocalNodeId) -> Json:
    """Return one JSON value for one LocalNodeId."""
    return {
        "id": value.id,
    }


def from_json_local_node_id(value: Json) -> LocalNodeId:
    """Return one LocalNodeId from one JSON value."""
    object_ = json_object(value)

    return LocalNodeId(
        id=json_int(json_field(object_, "id")),
    )


__all__ = [
    "LocalNodeId",
    "encode_local_node_id",
    "decode_local_node_id",
    "to_json_local_node_id",
    "from_json_local_node_id",
]
