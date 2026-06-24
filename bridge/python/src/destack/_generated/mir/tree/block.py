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
    json_object,
    json_optional,
)

import destack._generated.core.string
import destack._generated.mir.tree.node
import destack._generated.mir.tree.parameter


@dataclass(frozen=True, slots=True)
class Block:
    """A basic block is a sequence of instructions with."""

    # optional explicit block label
    name: destack._generated.core.string.StringId | None
    # SSA parameters passed from predecessor blocks
    parameters: Sequence[destack._generated.mir.tree.parameter.BlockParameter]
    # instructions in execution order
    instructions: Sequence[destack._generated.mir.tree.node.LocalNodeId]
    # how control flow leaves this block
    terminator: destack._generated.mir.tree.node.LocalNodeId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_block(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Block:
        """Decode one Block."""
        return decode_block(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_block(self)

    @classmethod
    def from_json(cls, value: Json) -> Block:
        """Return one Block from one JSON value."""
        return from_json_block(value)


def encode_block(writer: BinaryWriter, value: Block) -> None:
    """Encode one Block."""
    if value.name is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.core.string.encode_string_id(writer, value.name)
    writer.write_unsigned(len(value.parameters))
    for item_value_parameters_0 in value.parameters:
        destack._generated.mir.tree.parameter.encode_block_parameter(
            writer, item_value_parameters_0
        )
    writer.write_unsigned(len(value.instructions))
    for item_value_instructions_0 in value.instructions:
        destack._generated.mir.tree.node.encode_local_node_id(
            writer, item_value_instructions_0
        )
    destack._generated.mir.tree.node.encode_local_node_id(writer, value.terminator)


def decode_block(reader: BinaryReader) -> Block:
    """Decode one Block."""
    name = reader.read_option(
        lambda: destack._generated.core.string.decode_string_id(reader)
    )
    parameters = [
        destack._generated.mir.tree.parameter.decode_block_parameter(reader)
        for _ in range(reader.read_number())
    ]
    instructions = [
        destack._generated.mir.tree.node.decode_local_node_id(reader)
        for _ in range(reader.read_number())
    ]
    terminator = destack._generated.mir.tree.node.decode_local_node_id(reader)

    return Block(
        name=name,
        parameters=parameters,
        instructions=instructions,
        terminator=terminator,
    )


def to_json_block(value: Block) -> Json:
    """Return one JSON value for one Block."""
    return {
        **(
            {}
            if value.name is None
            else {"name": destack._generated.core.string.to_json_string_id(value.name)}
        ),
        "parameters": [
            destack._generated.mir.tree.parameter.to_json_block_parameter(item_0)
            for item_0 in value.parameters
        ],
        "instructions": [
            destack._generated.mir.tree.node.to_json_local_node_id(item_0)
            for item_0 in value.instructions
        ],
        "terminator": destack._generated.mir.tree.node.to_json_local_node_id(
            value.terminator
        ),
    }


def from_json_block(value: Json) -> Block:
    """Return one Block from one JSON value."""
    object_ = json_object(value)

    return Block(
        name=json_optional(
            object_,
            "name",
            lambda value: destack._generated.core.string.from_json_string_id(value),
        ),
        parameters=[
            destack._generated.mir.tree.parameter.from_json_block_parameter(item_0)
            for item_0 in json_array(json_field(object_, "parameters"))
        ],
        instructions=[
            destack._generated.mir.tree.node.from_json_local_node_id(item_0)
            for item_0 in json_array(json_field(object_, "instructions"))
        ],
        terminator=destack._generated.mir.tree.node.from_json_local_node_id(
            json_field(object_, "terminator")
        ),
    )


__all__ = [
    "Block",
    "encode_block",
    "decode_block",
    "to_json_block",
    "from_json_block",
]
