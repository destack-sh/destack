# generated bridge target, do not edit

from __future__ import annotations

import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_string,
)

"""Kind of a symbol in navigation queries."""
SymbolKind: typing.TypeAlias = (
    typing.Literal["file"]
    | typing.Literal["module"]
    | typing.Literal["namespace"]
    | typing.Literal["package"]
    | typing.Literal["class"]
    | typing.Literal["method"]
    | typing.Literal["property"]
    | typing.Literal["field"]
    | typing.Literal["constructor"]
    | typing.Literal["enum"]
    | typing.Literal["interface"]
    | typing.Literal["function"]
    | typing.Literal["variable"]
    | typing.Literal["constant"]
    | typing.Literal["string"]
    | typing.Literal["number"]
    | typing.Literal["boolean"]
    | typing.Literal["array"]
    | typing.Literal["object"]
    | typing.Literal["key"]
    | typing.Literal["null"]
    | typing.Literal["enumMember"]
    | typing.Literal["struct"]
    | typing.Literal["event"]
    | typing.Literal["operator"]
    | typing.Literal["typeParameter"]
)


def encode_symbol_kind(writer: BinaryWriter, value: SymbolKind) -> None:
    """Encode one SymbolKind."""
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


def decode_symbol_kind(reader: BinaryReader) -> SymbolKind:
    """Decode one SymbolKind."""
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


def to_json_symbol_kind(value: SymbolKind) -> Json:
    """Return one JSON value for one SymbolKind."""
    return value


def from_json_symbol_kind(value: Json) -> SymbolKind:
    """Return one SymbolKind from one JSON value."""
    variant = json_string(value)

    if variant == "file":
        return "file"
    elif variant == "module":
        return "module"
    elif variant == "namespace":
        return "namespace"
    elif variant == "package":
        return "package"
    elif variant == "class":
        return "class"
    elif variant == "method":
        return "method"
    elif variant == "property":
        return "property"
    elif variant == "field":
        return "field"
    elif variant == "constructor":
        return "constructor"
    elif variant == "enum":
        return "enum"
    elif variant == "interface":
        return "interface"
    elif variant == "function":
        return "function"
    elif variant == "variable":
        return "variable"
    elif variant == "constant":
        return "constant"
    elif variant == "string":
        return "string"
    elif variant == "number":
        return "number"
    elif variant == "boolean":
        return "boolean"
    elif variant == "array":
        return "array"
    elif variant == "object":
        return "object"
    elif variant == "key":
        return "key"
    elif variant == "null":
        return "null"
    elif variant == "enumMember":
        return "enumMember"
    elif variant == "struct":
        return "struct"
    elif variant == "event":
        return "event"
    elif variant == "operator":
        return "operator"
    elif variant == "typeParameter":
        return "typeParameter"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


__all__ = [
    "SymbolKind",
    "encode_symbol_kind",
    "decode_symbol_kind",
    "to_json_symbol_kind",
    "from_json_symbol_kind",
]
