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

import destack._generated.js.tree.node


@dataclass(frozen=True, slots=True)
class Block:
    """Block of statements."""

    # the statements in the block
    statements: Sequence[destack._generated.js.tree.node.LocalNodeId]

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
    writer.write_unsigned(len(value.statements))
    for item_value_statements_0 in value.statements:
        destack._generated.js.tree.node.encode_local_node_id(
            writer, item_value_statements_0
        )


def decode_block(reader: BinaryReader) -> Block:
    """Decode one Block."""
    statements = [
        destack._generated.js.tree.node.decode_local_node_id(reader)
        for _ in range(reader.read_number())
    ]

    return Block(
        statements=statements,
    )


def to_json_block(value: Block) -> Json:
    """Return one JSON value for one Block."""
    return {
        "statements": [
            destack._generated.js.tree.node.to_json_local_node_id(item_0)
            for item_0 in value.statements
        ],
    }


def from_json_block(value: Json) -> Block:
    """Return one Block from one JSON value."""
    object_ = json_object(value)

    return Block(
        statements=[
            destack._generated.js.tree.node.from_json_local_node_id(item_0)
            for item_0 in json_array(json_field(object_, "statements"))
        ],
    )


@dataclass(frozen=True, slots=True)
class CatchClause:
    """A catch clause."""

    # the optional catch pattern
    pattern: destack._generated.js.tree.node.LocalNodeId | None
    # the catch body
    body: destack._generated.js.tree.node.LocalNodeId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_catch_clause(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> CatchClause:
        """Decode one CatchClause."""
        return decode_catch_clause(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_catch_clause(self)

    @classmethod
    def from_json(cls, value: Json) -> CatchClause:
        """Return one CatchClause from one JSON value."""
        return from_json_catch_clause(value)


def encode_catch_clause(writer: BinaryWriter, value: CatchClause) -> None:
    """Encode one CatchClause."""
    if value.pattern is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.pattern)
    destack._generated.js.tree.node.encode_local_node_id(writer, value.body)


def decode_catch_clause(reader: BinaryReader) -> CatchClause:
    """Decode one CatchClause."""
    pattern = reader.read_option(
        lambda: destack._generated.js.tree.node.decode_local_node_id(reader)
    )
    body = destack._generated.js.tree.node.decode_local_node_id(reader)

    return CatchClause(
        pattern=pattern,
        body=body,
    )


def to_json_catch_clause(value: CatchClause) -> Json:
    """Return one JSON value for one CatchClause."""
    return {
        **(
            {}
            if value.pattern is None
            else {
                "pattern": destack._generated.js.tree.node.to_json_local_node_id(
                    value.pattern
                )
            }
        ),
        "body": destack._generated.js.tree.node.to_json_local_node_id(value.body),
    }


def from_json_catch_clause(value: Json) -> CatchClause:
    """Return one CatchClause from one JSON value."""
    object_ = json_object(value)

    return CatchClause(
        pattern=json_optional(
            object_,
            "pattern",
            lambda value: destack._generated.js.tree.node.from_json_local_node_id(
                value
            ),
        ),
        body=destack._generated.js.tree.node.from_json_local_node_id(
            json_field(object_, "body")
        ),
    )


@dataclass(frozen=True, slots=True)
class SwitchCase:
    """A switch case."""

    # the optional case selector
    value: destack._generated.js.tree.node.LocalNodeId | None
    # the body of the case
    body: destack._generated.js.tree.node.LocalNodeId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_switch_case(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> SwitchCase:
        """Decode one SwitchCase."""
        return decode_switch_case(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_switch_case(self)

    @classmethod
    def from_json(cls, value: Json) -> SwitchCase:
        """Return one SwitchCase from one JSON value."""
        return from_json_switch_case(value)


def encode_switch_case(writer: BinaryWriter, value: SwitchCase) -> None:
    """Encode one SwitchCase."""
    if value.value is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.value)
    destack._generated.js.tree.node.encode_local_node_id(writer, value.body)


def decode_switch_case(reader: BinaryReader) -> SwitchCase:
    """Decode one SwitchCase."""
    value_ = reader.read_option(
        lambda: destack._generated.js.tree.node.decode_local_node_id(reader)
    )
    body = destack._generated.js.tree.node.decode_local_node_id(reader)

    return SwitchCase(
        value=value_,
        body=body,
    )


def to_json_switch_case(value: SwitchCase) -> Json:
    """Return one JSON value for one SwitchCase."""
    return {
        **(
            {}
            if value.value is None
            else {
                "value": destack._generated.js.tree.node.to_json_local_node_id(
                    value.value
                )
            }
        ),
        "body": destack._generated.js.tree.node.to_json_local_node_id(value.body),
    }


def from_json_switch_case(value: Json) -> SwitchCase:
    """Return one SwitchCase from one JSON value."""
    object_ = json_object(value)

    return SwitchCase(
        value=json_optional(
            object_,
            "value",
            lambda value: destack._generated.js.tree.node.from_json_local_node_id(
                value
            ),
        ),
        body=destack._generated.js.tree.node.from_json_local_node_id(
            json_field(object_, "body")
        ),
    )


__all__ = [
    "Block",
    "encode_block",
    "decode_block",
    "to_json_block",
    "from_json_block",
    "CatchClause",
    "encode_catch_clause",
    "decode_catch_clause",
    "to_json_catch_clause",
    "from_json_catch_clause",
    "SwitchCase",
    "encode_switch_case",
    "decode_switch_case",
    "to_json_switch_case",
    "from_json_switch_case",
]
