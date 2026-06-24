# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_field,
    json_object,
    json_optional,
)

import destack._generated.js.tree.node


@dataclass(frozen=True, slots=True)
class Declarator:
    """A Declarator is an individual variable declaration within a let/const/var statement."""

    # the pattern to bind (can be a simple identifier or destructuring pattern)
    pattern: destack._generated.js.tree.node.LocalNodeId
    # optional type annotation
    ty: destack._generated.js.tree.node.LocalNodeId | None
    # optional value expression
    value: destack._generated.js.tree.node.LocalNodeId | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_declarator(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Declarator:
        """Decode one Declarator."""
        return decode_declarator(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_declarator(self)

    @classmethod
    def from_json(cls, value: Json) -> Declarator:
        """Return one Declarator from one JSON value."""
        return from_json_declarator(value)


def encode_declarator(writer: BinaryWriter, value: Declarator) -> None:
    """Encode one Declarator."""
    destack._generated.js.tree.node.encode_local_node_id(writer, value.pattern)
    if value.ty is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.ty)
    if value.value is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.value)


def decode_declarator(reader: BinaryReader) -> Declarator:
    """Decode one Declarator."""
    pattern = destack._generated.js.tree.node.decode_local_node_id(reader)
    ty = reader.read_option(
        lambda: destack._generated.js.tree.node.decode_local_node_id(reader)
    )
    value_ = reader.read_option(
        lambda: destack._generated.js.tree.node.decode_local_node_id(reader)
    )

    return Declarator(
        pattern=pattern,
        ty=ty,
        value=value_,
    )


def to_json_declarator(value: Declarator) -> Json:
    """Return one JSON value for one Declarator."""
    return {
        "pattern": destack._generated.js.tree.node.to_json_local_node_id(value.pattern),
        **(
            {}
            if value.ty is None
            else {"ty": destack._generated.js.tree.node.to_json_local_node_id(value.ty)}
        ),
        **(
            {}
            if value.value is None
            else {
                "value": destack._generated.js.tree.node.to_json_local_node_id(
                    value.value
                )
            }
        ),
    }


def from_json_declarator(value: Json) -> Declarator:
    """Return one Declarator from one JSON value."""
    object_ = json_object(value)

    return Declarator(
        pattern=destack._generated.js.tree.node.from_json_local_node_id(
            json_field(object_, "pattern")
        ),
        ty=json_optional(
            object_,
            "ty",
            lambda value: destack._generated.js.tree.node.from_json_local_node_id(
                value
            ),
        ),
        value=json_optional(
            object_,
            "value",
            lambda value: destack._generated.js.tree.node.from_json_local_node_id(
                value
            ),
        ),
    )


__all__ = [
    "Declarator",
    "encode_declarator",
    "decode_declarator",
    "to_json_declarator",
    "from_json_declarator",
]
