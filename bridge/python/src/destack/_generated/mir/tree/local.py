# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_field,
    json_object,
)

import destack._generated.mir.tree.node
import destack._generated.mir.tree.type


@dataclass(frozen=True, slots=True)
class Local:
    """Local variable (stack slot) in a function."""

    # the type of the value stored in this slot
    ty: destack._generated.mir.tree.node.LocalNodeId
    # whether this local can be mutated after initialization
    mutability: destack._generated.mir.tree.type.Mutability

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_local(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Local:
        """Decode one Local."""
        return decode_local(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_local(self)

    @classmethod
    def from_json(cls, value: Json) -> Local:
        """Return one Local from one JSON value."""
        return from_json_local(value)


def encode_local(writer: BinaryWriter, value: Local) -> None:
    """Encode one Local."""
    destack._generated.mir.tree.node.encode_local_node_id(writer, value.ty)
    destack._generated.mir.tree.type.encode_mutability(writer, value.mutability)


def decode_local(reader: BinaryReader) -> Local:
    """Decode one Local."""
    ty = destack._generated.mir.tree.node.decode_local_node_id(reader)
    mutability = destack._generated.mir.tree.type.decode_mutability(reader)

    return Local(
        ty=ty,
        mutability=mutability,
    )


def to_json_local(value: Local) -> Json:
    """Return one JSON value for one Local."""
    return {
        "ty": destack._generated.mir.tree.node.to_json_local_node_id(value.ty),
        "mutability": destack._generated.mir.tree.type.to_json_mutability(
            value.mutability
        ),
    }


def from_json_local(value: Json) -> Local:
    """Return one Local from one JSON value."""
    object_ = json_object(value)

    return Local(
        ty=destack._generated.mir.tree.node.from_json_local_node_id(
            json_field(object_, "ty")
        ),
        mutability=destack._generated.mir.tree.type.from_json_mutability(
            json_field(object_, "mutability")
        ),
    )


__all__ = [
    "Local",
    "encode_local",
    "decode_local",
    "to_json_local",
    "from_json_local",
]
