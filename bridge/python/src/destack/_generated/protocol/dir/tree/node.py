# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes

import destack._generated.protocol.source.file.model.module

if TYPE_CHECKING:
    from destack._generated.protocol.source.file.model.module import (
        ModuleId,
    )


@dataclass(frozen=True, slots=True)
class GlobalNodeIdAny:
    """Global node id across modules."""

    """The module id of the global node."""
    module_id: ModuleId
    """The local id of the global node."""
    local_id: LocalNodeIdAny


def encode_global_node_id_any(writer: Writer, value: GlobalNodeIdAny) -> None:
    destack._generated.protocol.source.file.model.module.encode_module_id(
        writer, value.module_id
    )
    encode_local_node_id_any(writer, value.local_id)


def decode_global_node_id_any(reader: Reader) -> GlobalNodeIdAny:
    field_0 = destack._generated.protocol.source.file.model.module.decode_module_id(
        reader
    )
    field_1 = decode_local_node_id_any(reader)

    return GlobalNodeIdAny(
        module_id=field_0,
        local_id=field_1,
    )


@dataclass(frozen=True, slots=True)
class LocalNodeIdAny:
    """Unique identifier for nodes with dynamic type in a local arena."""

    id: int
    ty: NodeType


def encode_local_node_id_any(writer: Writer, value: LocalNodeIdAny) -> None:
    writer.write_unsigned(value.id)
    encode_node_type(writer, value.ty)


def decode_local_node_id_any(reader: Reader) -> LocalNodeIdAny:
    field_0 = reader.read_number()
    field_1 = decode_node_type(reader)

    return LocalNodeIdAny(
        id=field_0,
        ty=field_1,
    )


"""The type of a node."""
NodeType: TypeAlias = (
    Literal["expression"]
    | Literal["typeExpression"]
    | Literal["block"]
    | Literal["catch"]
    | Literal["declaration"]
    | Literal["declarator"]
    | Literal["property"]
    | Literal["typeMember"]
    | Literal["typeMappedParameter"]
    | Literal["member"]
    | Literal["enumField"]
    | Literal["whereClause"]
    | Literal["dependencyItem"]
    | Literal["genericParameter"]
    | Literal["parameter"]
    | Literal["genericArgument"]
    | Literal["tupleElement"]
    | Literal["argument"]
    | Literal["matchCase"]
    | Literal["pattern"]
    | Literal["patternField"]
    | Literal["assignPattern"]
    | Literal["assignPatternField"]
    | Literal["decorator"]
)


def encode_node_type(writer: Writer, value: NodeType) -> None:
    if value == "expression":
        writer.write_unsigned(0)
    elif value == "typeExpression":
        writer.write_unsigned(1)
    elif value == "block":
        writer.write_unsigned(2)
    elif value == "catch":
        writer.write_unsigned(3)
    elif value == "declaration":
        writer.write_unsigned(4)
    elif value == "declarator":
        writer.write_unsigned(5)
    elif value == "property":
        writer.write_unsigned(6)
    elif value == "typeMember":
        writer.write_unsigned(7)
    elif value == "typeMappedParameter":
        writer.write_unsigned(8)
    elif value == "member":
        writer.write_unsigned(9)
    elif value == "enumField":
        writer.write_unsigned(10)
    elif value == "whereClause":
        writer.write_unsigned(11)
    elif value == "dependencyItem":
        writer.write_unsigned(12)
    elif value == "genericParameter":
        writer.write_unsigned(13)
    elif value == "parameter":
        writer.write_unsigned(14)
    elif value == "genericArgument":
        writer.write_unsigned(15)
    elif value == "tupleElement":
        writer.write_unsigned(16)
    elif value == "argument":
        writer.write_unsigned(17)
    elif value == "matchCase":
        writer.write_unsigned(18)
    elif value == "pattern":
        writer.write_unsigned(19)
    elif value == "patternField":
        writer.write_unsigned(20)
    elif value == "assignPattern":
        writer.write_unsigned(21)
    elif value == "assignPatternField":
        writer.write_unsigned(22)
    elif value == "decorator":
        writer.write_unsigned(23)
    else:
        raise SerdeError("unknown enum variant")


def decode_node_type(reader: Reader) -> NodeType:
    variant = reader.read_number()

    if variant == 0:
        return "expression"
    elif variant == 1:
        return "typeExpression"
    elif variant == 2:
        return "block"
    elif variant == 3:
        return "catch"
    elif variant == 4:
        return "declaration"
    elif variant == 5:
        return "declarator"
    elif variant == 6:
        return "property"
    elif variant == 7:
        return "typeMember"
    elif variant == 8:
        return "typeMappedParameter"
    elif variant == 9:
        return "member"
    elif variant == 10:
        return "enumField"
    elif variant == 11:
        return "whereClause"
    elif variant == 12:
        return "dependencyItem"
    elif variant == 13:
        return "genericParameter"
    elif variant == 14:
        return "parameter"
    elif variant == 15:
        return "genericArgument"
    elif variant == 16:
        return "tupleElement"
    elif variant == 17:
        return "argument"
    elif variant == 18:
        return "matchCase"
    elif variant == 19:
        return "pattern"
    elif variant == 20:
        return "patternField"
    elif variant == 21:
        return "assignPattern"
    elif variant == 22:
        return "assignPatternField"
    elif variant == 23:
        return "decorator"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


__all__ = [
    "GlobalNodeIdAny",
    "encode_global_node_id_any",
    "decode_global_node_id_any",
    "LocalNodeIdAny",
    "encode_local_node_id_any",
    "decode_local_node_id_any",
    "NodeType",
    "encode_node_type",
    "decode_node_type",
]
