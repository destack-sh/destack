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
import destack._generated.mir.tree.value


@dataclass(frozen=True, slots=True)
class Call:
    """Shared call facts for one call-like instruction or terminator."""

    # the call arguments
    arguments: destack._generated.mir.tree.value.ValueSlice
    # the signature type for the callee
    signature: destack._generated.mir.tree.node.LocalNodeId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_call(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Call:
        """Decode one Call."""
        return decode_call(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_call(self)

    @classmethod
    def from_json(cls, value: Json) -> Call:
        """Return one Call from one JSON value."""
        return from_json_call(value)


def encode_call(writer: BinaryWriter, value: Call) -> None:
    """Encode one Call."""
    destack._generated.mir.tree.value.encode_value_slice(writer, value.arguments)
    destack._generated.mir.tree.node.encode_local_node_id(writer, value.signature)


def decode_call(reader: BinaryReader) -> Call:
    """Decode one Call."""
    arguments = destack._generated.mir.tree.value.decode_value_slice(reader)
    signature = destack._generated.mir.tree.node.decode_local_node_id(reader)

    return Call(
        arguments=arguments,
        signature=signature,
    )


def to_json_call(value: Call) -> Json:
    """Return one JSON value for one Call."""
    return {
        "arguments": destack._generated.mir.tree.value.to_json_value_slice(
            value.arguments
        ),
        "signature": destack._generated.mir.tree.node.to_json_local_node_id(
            value.signature
        ),
    }


def from_json_call(value: Json) -> Call:
    """Return one Call from one JSON value."""
    object_ = json_object(value)

    return Call(
        arguments=destack._generated.mir.tree.value.from_json_value_slice(
            json_field(object_, "arguments")
        ),
        signature=destack._generated.mir.tree.node.from_json_local_node_id(
            json_field(object_, "signature")
        ),
    )


__all__ = [
    "Call",
    "encode_call",
    "decode_call",
    "to_json_call",
    "from_json_call",
]
