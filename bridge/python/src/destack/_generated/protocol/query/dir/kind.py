# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes

"""Kind of a symbol in navigation queries."""
SymbolKind: TypeAlias = (
    Literal["file"]
    | Literal["module"]
    | Literal["namespace"]
    | Literal["package"]
    | Literal["class"]
    | Literal["method"]
    | Literal["property"]
    | Literal["field"]
    | Literal["constructor"]
    | Literal["enum"]
    | Literal["interface"]
    | Literal["function"]
    | Literal["variable"]
    | Literal["constant"]
    | Literal["string"]
    | Literal["number"]
    | Literal["boolean"]
    | Literal["array"]
    | Literal["object"]
    | Literal["key"]
    | Literal["null"]
    | Literal["enumMember"]
    | Literal["struct"]
    | Literal["event"]
    | Literal["operator"]
    | Literal["typeParameter"]
)


def encode_symbol_kind(writer: Writer, value: SymbolKind) -> None:
    if value == "file":
        writer.write_unsigned(0)
    elif value == "module":
        writer.write_unsigned(1)
    elif value == "namespace":
        writer.write_unsigned(2)
    elif value == "package":
        writer.write_unsigned(3)
    elif value == "class":
        writer.write_unsigned(4)
    elif value == "method":
        writer.write_unsigned(5)
    elif value == "property":
        writer.write_unsigned(6)
    elif value == "field":
        writer.write_unsigned(7)
    elif value == "constructor":
        writer.write_unsigned(8)
    elif value == "enum":
        writer.write_unsigned(9)
    elif value == "interface":
        writer.write_unsigned(10)
    elif value == "function":
        writer.write_unsigned(11)
    elif value == "variable":
        writer.write_unsigned(12)
    elif value == "constant":
        writer.write_unsigned(13)
    elif value == "string":
        writer.write_unsigned(14)
    elif value == "number":
        writer.write_unsigned(15)
    elif value == "boolean":
        writer.write_unsigned(16)
    elif value == "array":
        writer.write_unsigned(17)
    elif value == "object":
        writer.write_unsigned(18)
    elif value == "key":
        writer.write_unsigned(19)
    elif value == "null":
        writer.write_unsigned(20)
    elif value == "enumMember":
        writer.write_unsigned(21)
    elif value == "struct":
        writer.write_unsigned(22)
    elif value == "event":
        writer.write_unsigned(23)
    elif value == "operator":
        writer.write_unsigned(24)
    elif value == "typeParameter":
        writer.write_unsigned(25)
    else:
        raise SerdeError("unknown enum variant")


def decode_symbol_kind(reader: Reader) -> SymbolKind:
    variant = reader.read_number()

    if variant == 0:
        return "file"
    elif variant == 1:
        return "module"
    elif variant == 2:
        return "namespace"
    elif variant == 3:
        return "package"
    elif variant == 4:
        return "class"
    elif variant == 5:
        return "method"
    elif variant == 6:
        return "property"
    elif variant == 7:
        return "field"
    elif variant == 8:
        return "constructor"
    elif variant == 9:
        return "enum"
    elif variant == 10:
        return "interface"
    elif variant == 11:
        return "function"
    elif variant == 12:
        return "variable"
    elif variant == 13:
        return "constant"
    elif variant == 14:
        return "string"
    elif variant == 15:
        return "number"
    elif variant == 16:
        return "boolean"
    elif variant == 17:
        return "array"
    elif variant == 18:
        return "object"
    elif variant == 19:
        return "key"
    elif variant == 20:
        return "null"
    elif variant == 21:
        return "enumMember"
    elif variant == 22:
        return "struct"
    elif variant == 23:
        return "event"
    elif variant == 24:
        return "operator"
    elif variant == 25:
        return "typeParameter"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


__all__ = [
    "SymbolKind",
    "encode_symbol_kind",
    "decode_symbol_kind",
]
