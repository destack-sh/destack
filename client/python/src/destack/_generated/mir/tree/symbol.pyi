# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.core.string

@dataclass(frozen=True, slots=True)
class Symbol:
    """Persistent, mangled identity of a function, global, or type."""

    value: destack._generated.core.string.StringId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Symbol: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Symbol: ...

def encode_symbol(writer: BinaryWriter, value: Symbol) -> None: ...
def decode_symbol(reader: BinaryReader) -> Symbol: ...
def to_json_symbol(value: Symbol) -> Json: ...
def from_json_symbol(value: Json) -> Symbol: ...

__all__ = [
    "Symbol",
    "encode_symbol",
    "decode_symbol",
    "to_json_symbol",
    "from_json_symbol",
]
