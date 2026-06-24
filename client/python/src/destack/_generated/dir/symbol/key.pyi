# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

from destack._impl.dir.symbol.key import (
    StaticKeyImpl,
)

import destack._generated.core.string
import destack._generated.dir.symbol.symbol

@dataclass(frozen=True, slots=True)
class StaticKeyName(StaticKeyImpl):
    """Regular name key (like `x` or `"weird identifier"`)."""

    name: destack._generated.core.string.StringId
    kind: typing.Literal["name"] = "name"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class StaticKeyIndex(StaticKeyImpl):
    """Positional index key (like `0` or `1`)."""

    index: int
    kind: typing.Literal["index"] = "index"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class StaticKeySymbol(StaticKeyImpl):
    """Symbol key."""

    symbol: SymbolKey
    kind: typing.Literal["symbol"] = "symbol"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Key for some static "identifier" (name, positional index, symbol)."""
StaticKey: typing.TypeAlias = StaticKeyName | StaticKeyIndex | StaticKeySymbol

def encode_static_key(writer: BinaryWriter, value: StaticKey) -> None: ...
def decode_static_key(reader: BinaryReader) -> StaticKey: ...
def to_json_static_key(value: StaticKey) -> Json: ...
def from_json_static_key(value: Json) -> StaticKey: ...

@dataclass(frozen=True, slots=True)
class SymbolKeyUnique:
    """Unique symbol key from a declaration."""

    unique: destack._generated.dir.symbol.symbol.GlobalSymbolId
    kind: typing.Literal["unique"] = "unique"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class SymbolKeyRegistry:
    """Symbol.for registry key (string is the content of `Symbol.for`)."""

    registry: destack._generated.core.string.StringId
    kind: typing.Literal["registry"] = "registry"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Symbol as a key."""
SymbolKey: typing.TypeAlias = SymbolKeyUnique | SymbolKeyRegistry

def encode_symbol_key(writer: BinaryWriter, value: SymbolKey) -> None: ...
def decode_symbol_key(reader: BinaryReader) -> SymbolKey: ...
def to_json_symbol_key(value: SymbolKey) -> Json: ...
def from_json_symbol_key(value: Json) -> SymbolKey: ...

__all__ = [
    "StaticKey",
    "encode_static_key",
    "decode_static_key",
    "to_json_static_key",
    "from_json_static_key",
    "StaticKeyName",
    "StaticKeyIndex",
    "StaticKeySymbol",
    "SymbolKey",
    "encode_symbol_key",
    "decode_symbol_key",
    "to_json_symbol_key",
    "from_json_symbol_key",
    "SymbolKeyUnique",
    "SymbolKeyRegistry",
]
