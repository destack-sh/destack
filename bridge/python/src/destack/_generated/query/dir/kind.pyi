# generated bridge target, do not edit

from __future__ import annotations

import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

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

def encode_symbol_kind(writer: BinaryWriter, value: SymbolKind) -> None: ...
def decode_symbol_kind(reader: BinaryReader) -> SymbolKind: ...
def to_json_symbol_kind(value: SymbolKind) -> Json: ...
def from_json_symbol_kind(value: Json) -> SymbolKind: ...

__all__ = [
    "SymbolKind",
    "encode_symbol_kind",
    "decode_symbol_kind",
    "to_json_symbol_kind",
    "from_json_symbol_kind",
]
