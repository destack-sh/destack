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


"""A Mutability is the mutability of a binding (const or mutable)."""
Mutability: typing.TypeAlias = typing.Literal["immutable"] | typing.Literal["mutable"]


def encode_mutability(writer: BinaryWriter, value: Mutability) -> None:
    """Encode one Mutability."""
    if value == "immutable":
        writer.write_unsigned(0)
    elif value == "mutable":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_mutability(reader: BinaryReader) -> Mutability:
    """Decode one Mutability."""
    variant = reader.read_number()

    if variant == 0:
        return "immutable"
    elif variant == 1:
        return "mutable"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_mutability(value: Mutability) -> Json:
    """Return one JSON value for one Mutability."""
    return value


def from_json_mutability(value: Json) -> Mutability:
    """Return one Mutability from one JSON value."""
    variant = json_string(value)

    if variant == "immutable":
        return "immutable"
    elif variant == "mutable":
        return "mutable"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""The asynchrony of a function."""
Asynchrony: typing.TypeAlias = typing.Literal["sync"] | typing.Literal["async"]


def encode_asynchrony(writer: BinaryWriter, value: Asynchrony) -> None:
    """Encode one Asynchrony."""
    if value == "sync":
        writer.write_unsigned(0)
    elif value == "async":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_asynchrony(reader: BinaryReader) -> Asynchrony:
    """Decode one Asynchrony."""
    variant = reader.read_number()

    if variant == 0:
        return "sync"
    elif variant == 1:
        return "async"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_asynchrony(value: Asynchrony) -> Json:
    """Return one JSON value for one Asynchrony."""
    return value


def from_json_asynchrony(value: Json) -> Asynchrony:
    """Return one Asynchrony from one JSON value."""
    variant = json_string(value)

    if variant == "sync":
        return "sync"
    elif variant == "async":
        return "async"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""A Visibility is the visibility of an item."""
Visibility: typing.TypeAlias = (
    typing.Literal["public"] | typing.Literal["protected"] | typing.Literal["private"]
)


def encode_visibility(writer: BinaryWriter, value: Visibility) -> None:
    """Encode one Visibility."""
    if value == "public":
        writer.write_unsigned(0)
    elif value == "protected":
        writer.write_unsigned(1)
    elif value == "private":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_visibility(reader: BinaryReader) -> Visibility:
    """Decode one Visibility."""
    variant = reader.read_number()

    if variant == 0:
        return "public"
    elif variant == 1:
        return "protected"
    elif variant == 2:
        return "private"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_visibility(value: Visibility) -> Json:
    """Return one JSON value for one Visibility."""
    return value


def from_json_visibility(value: Json) -> Visibility:
    """Return one Visibility from one JSON value."""
    variant = json_string(value)

    if variant == "public":
        return "public"
    elif variant == "protected":
        return "protected"
    elif variant == "private":
        return "private"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class LocalNodeIdAny:
    """Unique identifier for nodes with dynamic type."""

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


"""The type of a node."""
NodeType: typing.TypeAlias = (
    typing.Literal["block"]
    | typing.Literal["catchClause"]
    | typing.Literal["statement"]
    | typing.Literal["expression"]
    | typing.Literal["arrayElement"]
    | typing.Literal["declaration"]
    | typing.Literal["declarator"]
    | typing.Literal["property"]
    | typing.Literal["member"]
    | typing.Literal["typeExpression"]
    | typing.Literal["tupleElement"]
    | typing.Literal["typeMember"]
    | typing.Literal["enumField"]
    | typing.Literal["dependencyItem"]
    | typing.Literal["switchCase"]
    | typing.Literal["pattern"]
    | typing.Literal["patternField"]
    | typing.Literal["assignPattern"]
    | typing.Literal["assignPatternField"]
    | typing.Literal["genericParameter"]
    | typing.Literal["parameter"]
    | typing.Literal["argument"]
    | typing.Literal["annotation"]
)


def encode_node_type(writer: BinaryWriter, value: NodeType) -> None:
    """Encode one NodeType."""
    if value == "block":
        writer.write_unsigned(0)
    elif value == "catchClause":
        writer.write_unsigned(1)
    elif value == "statement":
        writer.write_unsigned(2)
    elif value == "expression":
        writer.write_unsigned(3)
    elif value == "arrayElement":
        writer.write_unsigned(4)
    elif value == "declaration":
        writer.write_unsigned(5)
    elif value == "declarator":
        writer.write_unsigned(6)
    elif value == "property":
        writer.write_unsigned(7)
    elif value == "member":
        writer.write_unsigned(8)
    elif value == "typeExpression":
        writer.write_unsigned(9)
    elif value == "tupleElement":
        writer.write_unsigned(10)
    elif value == "typeMember":
        writer.write_unsigned(11)
    elif value == "enumField":
        writer.write_unsigned(12)
    elif value == "dependencyItem":
        writer.write_unsigned(13)
    elif value == "switchCase":
        writer.write_unsigned(14)
    elif value == "pattern":
        writer.write_unsigned(15)
    elif value == "patternField":
        writer.write_unsigned(16)
    elif value == "assignPattern":
        writer.write_unsigned(17)
    elif value == "assignPatternField":
        writer.write_unsigned(18)
    elif value == "genericParameter":
        writer.write_unsigned(19)
    elif value == "parameter":
        writer.write_unsigned(20)
    elif value == "argument":
        writer.write_unsigned(21)
    elif value == "annotation":
        writer.write_unsigned(22)
    else:
        raise SerdeError("unknown enum variant")


def decode_node_type(reader: BinaryReader) -> NodeType:
    """Decode one NodeType."""
    variant = reader.read_number()

    if variant == 0:
        return "block"
    elif variant == 1:
        return "catchClause"
    elif variant == 2:
        return "statement"
    elif variant == 3:
        return "expression"
    elif variant == 4:
        return "arrayElement"
    elif variant == 5:
        return "declaration"
    elif variant == 6:
        return "declarator"
    elif variant == 7:
        return "property"
    elif variant == 8:
        return "member"
    elif variant == 9:
        return "typeExpression"
    elif variant == 10:
        return "tupleElement"
    elif variant == 11:
        return "typeMember"
    elif variant == 12:
        return "enumField"
    elif variant == 13:
        return "dependencyItem"
    elif variant == 14:
        return "switchCase"
    elif variant == 15:
        return "pattern"
    elif variant == 16:
        return "patternField"
    elif variant == 17:
        return "assignPattern"
    elif variant == 18:
        return "assignPatternField"
    elif variant == 19:
        return "genericParameter"
    elif variant == 20:
        return "parameter"
    elif variant == 21:
        return "argument"
    elif variant == 22:
        return "annotation"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_node_type(value: NodeType) -> Json:
    """Return one JSON value for one NodeType."""
    return value


def from_json_node_type(value: Json) -> NodeType:
    """Return one NodeType from one JSON value."""
    variant = json_string(value)

    if variant == "block":
        return "block"
    elif variant == "catchClause":
        return "catchClause"
    elif variant == "statement":
        return "statement"
    elif variant == "expression":
        return "expression"
    elif variant == "arrayElement":
        return "arrayElement"
    elif variant == "declaration":
        return "declaration"
    elif variant == "declarator":
        return "declarator"
    elif variant == "property":
        return "property"
    elif variant == "member":
        return "member"
    elif variant == "typeExpression":
        return "typeExpression"
    elif variant == "tupleElement":
        return "tupleElement"
    elif variant == "typeMember":
        return "typeMember"
    elif variant == "enumField":
        return "enumField"
    elif variant == "dependencyItem":
        return "dependencyItem"
    elif variant == "switchCase":
        return "switchCase"
    elif variant == "pattern":
        return "pattern"
    elif variant == "patternField":
        return "patternField"
    elif variant == "assignPattern":
        return "assignPattern"
    elif variant == "assignPatternField":
        return "assignPatternField"
    elif variant == "genericParameter":
        return "genericParameter"
    elif variant == "parameter":
        return "parameter"
    elif variant == "argument":
        return "argument"
    elif variant == "annotation":
        return "annotation"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


__all__ = [
    "LocalNodeId",
    "encode_local_node_id",
    "decode_local_node_id",
    "to_json_local_node_id",
    "from_json_local_node_id",
    "Mutability",
    "encode_mutability",
    "decode_mutability",
    "to_json_mutability",
    "from_json_mutability",
    "Asynchrony",
    "encode_asynchrony",
    "decode_asynchrony",
    "to_json_asynchrony",
    "from_json_asynchrony",
    "Visibility",
    "encode_visibility",
    "decode_visibility",
    "to_json_visibility",
    "from_json_visibility",
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
