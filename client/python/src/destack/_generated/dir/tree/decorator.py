# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_field,
    json_object,
    json_string,
)

import destack._generated.dir.tree.node


@dataclass(frozen=True, slots=True)
class Decorator:
    """A decorator attached to an owner node."""

    # the decorator expression
    expression: destack._generated.dir.tree.node.LocalNodeId
    # the decorator position relative to its owner
    position: DecoratorPosition

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_decorator(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Decorator:
        """Decode one Decorator."""
        return decode_decorator(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_decorator(self)

    @classmethod
    def from_json(cls, value: Json) -> Decorator:
        """Return one Decorator from one JSON value."""
        return from_json_decorator(value)


def encode_decorator(writer: BinaryWriter, value: Decorator) -> None:
    """Encode one Decorator."""
    destack._generated.dir.tree.node.encode_local_node_id(writer, value.expression)
    encode_decorator_position(writer, value.position)


def decode_decorator(reader: BinaryReader) -> Decorator:
    """Decode one Decorator."""
    expression = destack._generated.dir.tree.node.decode_local_node_id(reader)
    position = decode_decorator_position(reader)

    return Decorator(
        expression=expression,
        position=position,
    )


def to_json_decorator(value: Decorator) -> Json:
    """Return one JSON value for one Decorator."""
    return {
        "expression": destack._generated.dir.tree.node.to_json_local_node_id(
            value.expression
        ),
        "position": to_json_decorator_position(value.position),
    }


def from_json_decorator(value: Json) -> Decorator:
    """Return one Decorator from one JSON value."""
    object_ = json_object(value)

    return Decorator(
        expression=destack._generated.dir.tree.node.from_json_local_node_id(
            json_field(object_, "expression")
        ),
        position=from_json_decorator_position(json_field(object_, "position")),
    )


"""The position of one decorator relative to its owner."""
DecoratorPosition: typing.TypeAlias = (
    typing.Literal["blockInfix"]
    | typing.Literal["blockPrefix"]
    | typing.Literal["blockPostfix"]
    | typing.Literal["linePrefix"]
    | typing.Literal["linePostfix"]
    | typing.Literal["linePostfixBoundary"]
)


def encode_decorator_position(writer: BinaryWriter, value: DecoratorPosition) -> None:
    """Encode one DecoratorPosition."""
    if value == "blockInfix":
        writer.write_unsigned(0)
    elif value == "blockPrefix":
        writer.write_unsigned(1)
    elif value == "blockPostfix":
        writer.write_unsigned(2)
    elif value == "linePrefix":
        writer.write_unsigned(3)
    elif value == "linePostfix":
        writer.write_unsigned(4)
    elif value == "linePostfixBoundary":
        writer.write_unsigned(5)
    else:
        raise SerdeError("unknown enum variant")


def decode_decorator_position(reader: BinaryReader) -> DecoratorPosition:
    """Decode one DecoratorPosition."""
    variant = reader.read_number()

    if variant == 0:
        return "blockInfix"
    elif variant == 1:
        return "blockPrefix"
    elif variant == 2:
        return "blockPostfix"
    elif variant == 3:
        return "linePrefix"
    elif variant == 4:
        return "linePostfix"
    elif variant == 5:
        return "linePostfixBoundary"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_decorator_position(value: DecoratorPosition) -> Json:
    """Return one JSON value for one DecoratorPosition."""
    return value


def from_json_decorator_position(value: Json) -> DecoratorPosition:
    """Return one DecoratorPosition from one JSON value."""
    variant = json_string(value)

    if variant == "blockInfix":
        return "blockInfix"
    elif variant == "blockPrefix":
        return "blockPrefix"
    elif variant == "blockPostfix":
        return "blockPostfix"
    elif variant == "linePrefix":
        return "linePrefix"
    elif variant == "linePostfix":
        return "linePostfix"
    elif variant == "linePostfixBoundary":
        return "linePostfixBoundary"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


__all__ = [
    "Decorator",
    "encode_decorator",
    "decode_decorator",
    "to_json_decorator",
    "from_json_decorator",
    "DecoratorPosition",
    "encode_decorator_position",
    "decode_decorator_position",
    "to_json_decorator_position",
    "from_json_decorator_position",
]
