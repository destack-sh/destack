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
    json_int,
    json_object,
    json_string,
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


@dataclass(frozen=True, slots=True)
class LocalNodeIdAny:
    """Unique identifier for nodes with dynamic type in a local arena."""

    id: int
    ty: NodeType

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_local_node_id_any(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> LocalNodeIdAny:
        """Decode one LocalNodeIdAny."""
        return decode_local_node_id_any(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_local_node_id_any(self)

    @classmethod
    def from_json(cls, value: Json) -> LocalNodeIdAny:
        """Return one LocalNodeIdAny from one JSON value."""
        return from_json_local_node_id_any(value)


def encode_local_node_id_any(writer: BinaryWriter, value: LocalNodeIdAny) -> None:
    """Encode one LocalNodeIdAny."""
    writer.write_unsigned(value.id)
    encode_node_type(writer, value.ty)


def decode_local_node_id_any(reader: BinaryReader) -> LocalNodeIdAny:
    """Decode one LocalNodeIdAny."""
    id = reader.read_number()
    ty = decode_node_type(reader)

    return LocalNodeIdAny(
        id=id,
        ty=ty,
    )


def to_json_local_node_id_any(value: LocalNodeIdAny) -> Json:
    """Return one JSON value for one LocalNodeIdAny."""
    return {
        "id": value.id,
        "ty": to_json_node_type(value.ty),
    }


def from_json_local_node_id_any(value: Json) -> LocalNodeIdAny:
    """Return one LocalNodeIdAny from one JSON value."""
    object_ = json_object(value)

    return LocalNodeIdAny(
        id=json_int(json_field(object_, "id")),
        ty=from_json_node_type(json_field(object_, "ty")),
    )


"""The type of a MIR node."""
NodeType: typing.TypeAlias = (
    typing.Literal["function"]
    | typing.Literal["block"]
    | typing.Literal["instruction"]
    | typing.Literal["terminator"]
    | typing.Literal["local"]
    | typing.Literal["type"]
    | typing.Literal["typeAlias"]
    | typing.Literal["field"]
    | typing.Literal["global"]
)


def encode_node_type(writer: BinaryWriter, value: NodeType) -> None:
    """Encode one NodeType."""
    if value == "function":
        writer.write_unsigned(0)
    elif value == "block":
        writer.write_unsigned(1)
    elif value == "instruction":
        writer.write_unsigned(2)
    elif value == "terminator":
        writer.write_unsigned(3)
    elif value == "local":
        writer.write_unsigned(4)
    elif value == "type":
        writer.write_unsigned(5)
    elif value == "typeAlias":
        writer.write_unsigned(6)
    elif value == "field":
        writer.write_unsigned(7)
    elif value == "global":
        writer.write_unsigned(8)
    else:
        raise SerdeError("unknown enum variant")


def decode_node_type(reader: BinaryReader) -> NodeType:
    """Decode one NodeType."""
    variant = reader.read_number()

    if variant == 0:
        return "function"
    elif variant == 1:
        return "block"
    elif variant == 2:
        return "instruction"
    elif variant == 3:
        return "terminator"
    elif variant == 4:
        return "local"
    elif variant == 5:
        return "type"
    elif variant == 6:
        return "typeAlias"
    elif variant == 7:
        return "field"
    elif variant == 8:
        return "global"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_node_type(value: NodeType) -> Json:
    """Return one JSON value for one NodeType."""
    return value


def from_json_node_type(value: Json) -> NodeType:
    """Return one NodeType from one JSON value."""
    variant = json_string(value)

    if variant == "function":
        return "function"
    elif variant == "block":
        return "block"
    elif variant == "instruction":
        return "instruction"
    elif variant == "terminator":
        return "terminator"
    elif variant == "local":
        return "local"
    elif variant == "type":
        return "type"
    elif variant == "typeAlias":
        return "typeAlias"
    elif variant == "field":
        return "field"
    elif variant == "global":
        return "global"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


__all__ = [
    "LocalNodeId",
    "encode_local_node_id",
    "decode_local_node_id",
    "to_json_local_node_id",
    "from_json_local_node_id",
    "LocalNodeIdAny",
    "encode_local_node_id_any",
    "decode_local_node_id_any",
    "to_json_local_node_id_any",
    "from_json_local_node_id_any",
    "NodeType",
    "encode_node_type",
    "decode_node_type",
    "to_json_node_type",
    "from_json_node_type",
]
