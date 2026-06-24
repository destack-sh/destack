# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_array,
    json_field,
    json_object,
    json_optional,
    json_string,
)

from destack._impl.dir.tree.block import (
    BlockImpl,
)

import destack._generated.dir.tree.node


@dataclass(frozen=True, slots=True)
class Block(BlockImpl):
    """A Block is a block of statements."""

    # the block context
    context: BlockContext
    # the structural form of the block
    form: BlockForm
    # the leading expressions whose values are discarded
    leading_expressions: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    # the optional tail expression whose value becomes the block value
    tail_expression: destack._generated.dir.tree.node.LocalNodeId | None

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
    encode_block_context(writer, value.context)
    encode_block_form(writer, value.form)
    writer.write_unsigned(len(value.leading_expressions))
    for item_value_leading_expressions_0 in value.leading_expressions:
        destack._generated.dir.tree.node.encode_local_node_id(
            writer, item_value_leading_expressions_0
        )
    if value.tail_expression is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.tree.node.encode_local_node_id(
            writer, value.tail_expression
        )


def decode_block(reader: BinaryReader) -> Block:
    """Decode one Block."""
    context = decode_block_context(reader)
    form = decode_block_form(reader)
    leading_expressions = [
        destack._generated.dir.tree.node.decode_local_node_id(reader)
        for _ in range(reader.read_number())
    ]
    tail_expression = reader.read_option(
        lambda: destack._generated.dir.tree.node.decode_local_node_id(reader)
    )

    return Block(
        context=context,
        form=form,
        leading_expressions=leading_expressions,
        tail_expression=tail_expression,
    )


def to_json_block(value: Block) -> Json:
    """Return one JSON value for one Block."""
    return {
        "context": to_json_block_context(value.context),
        "form": to_json_block_form(value.form),
        "leadingExpressions": [
            destack._generated.dir.tree.node.to_json_local_node_id(item_0)
            for item_0 in value.leading_expressions
        ],
        **(
            {}
            if value.tail_expression is None
            else {
                "tailExpression": destack._generated.dir.tree.node.to_json_local_node_id(
                    value.tail_expression
                )
            }
        ),
    }


def from_json_block(value: Json) -> Block:
    """Return one Block from one JSON value."""
    object_ = json_object(value)

    return Block(
        context=from_json_block_context(json_field(object_, "context")),
        form=from_json_block_form(json_field(object_, "form")),
        leading_expressions=[
            destack._generated.dir.tree.node.from_json_local_node_id(item_0)
            for item_0 in json_array(json_field(object_, "leadingExpressions"))
        ],
        tail_expression=json_optional(
            object_,
            "tailExpression",
            lambda value: destack._generated.dir.tree.node.from_json_local_node_id(
                value
            ),
        ),
    )


"""How a block is interpreted."""
BlockContext: typing.TypeAlias = (
    typing.Literal["expression"] | typing.Literal["statement"]
)


def encode_block_context(writer: BinaryWriter, value: BlockContext) -> None:
    """Encode one BlockContext."""
    if value == "expression":
        writer.write_unsigned(0)
    elif value == "statement":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_block_context(reader: BinaryReader) -> BlockContext:
    """Decode one BlockContext."""
    variant = reader.read_number()

    if variant == 0:
        return "expression"
    elif variant == 1:
        return "statement"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_block_context(value: BlockContext) -> Json:
    """Return one JSON value for one BlockContext."""
    return value


def from_json_block_context(value: Json) -> BlockContext:
    """Return one BlockContext from one JSON value."""
    variant = json_string(value)

    if variant == "expression":
        return "expression"
    elif variant == "statement":
        return "statement"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""The structural form of a block."""
BlockForm: typing.TypeAlias = (
    typing.Literal["explicit"] | typing.Literal["do"] | typing.Literal["implicit"]
)


def encode_block_form(writer: BinaryWriter, value: BlockForm) -> None:
    """Encode one BlockForm."""
    if value == "explicit":
        writer.write_unsigned(0)
    elif value == "do":
        writer.write_unsigned(1)
    elif value == "implicit":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_block_form(reader: BinaryReader) -> BlockForm:
    """Decode one BlockForm."""
    variant = reader.read_number()

    if variant == 0:
        return "explicit"
    elif variant == 1:
        return "do"
    elif variant == 2:
        return "implicit"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_block_form(value: BlockForm) -> Json:
    """Return one JSON value for one BlockForm."""
    return value


def from_json_block_form(value: Json) -> BlockForm:
    """Return one BlockForm from one JSON value."""
    variant = json_string(value)

    if variant == "explicit":
        return "explicit"
    elif variant == "do":
        return "do"
    elif variant == "implicit":
        return "implicit"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


__all__ = [
    "Block",
    "encode_block",
    "decode_block",
    "to_json_block",
    "from_json_block",
    "BlockContext",
    "encode_block_context",
    "decode_block_context",
    "to_json_block_context",
    "from_json_block_context",
    "BlockForm",
    "encode_block_form",
    "decode_block_form",
    "to_json_block_form",
    "from_json_block_form",
]
