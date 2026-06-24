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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_static_key(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_static_key(self)


@dataclass(frozen=True, slots=True)
class StaticKeyIndex(StaticKeyImpl):
    """Positional index key (like `0` or `1`)."""

    index: int
    kind: typing.Literal["index"] = "index"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_static_key(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_static_key(self)


@dataclass(frozen=True, slots=True)
class StaticKeySymbol(StaticKeyImpl):
    """Symbol key."""

    symbol: SymbolKey
    kind: typing.Literal["symbol"] = "symbol"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_static_key(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_static_key(self)


"""Key for some static "identifier" (name, positional index, symbol)."""
StaticKey: typing.TypeAlias = StaticKeyName | StaticKeyIndex | StaticKeySymbol


def encode_static_key(writer: BinaryWriter, value: StaticKey) -> None:
    """Encode one StaticKey."""
    if value.kind == "name":
        writer.write_unsigned(0)
        destack._generated.core.string.encode_string_id(writer, value.name)
    elif value.kind == "index":
        writer.write_unsigned(1)
        writer.write_unsigned(value.index)
    elif value.kind == "symbol":
        writer.write_unsigned(2)
        encode_symbol_key(writer, value.symbol)
    else:
        raise SerdeError("unknown enum variant")


def decode_static_key(reader: BinaryReader) -> StaticKey:
    """Decode one StaticKey."""
    variant = reader.read_number()

    if variant == 0:
        name = destack._generated.core.string.decode_string_id(reader)

        return StaticKeyName(name=name)
    elif variant == 1:
        index = reader.read_number()

        return StaticKeyIndex(index=index)
    elif variant == 2:
        symbol = decode_symbol_key(reader)

        return StaticKeySymbol(symbol=symbol)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_static_key(value: StaticKey) -> Json:
    """Return one JSON value for one StaticKey."""
    if value.kind == "name":
        return {
            "kind": "name",
            "name": destack._generated.core.string.to_json_string_id(value.name),
        }
    elif value.kind == "index":
        return {
            "kind": "index",
            "index": value.index,
        }
    elif value.kind == "symbol":
        return {
            "kind": "symbol",
            "symbol": to_json_symbol_key(value.symbol),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_static_key(value: Json) -> StaticKey:
    """Return one StaticKey from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "name":
        return StaticKeyName(
            name=destack._generated.core.string.from_json_string_id(
                json_field(object_, "name")
            )
        )
    elif kind == "index":
        return StaticKeyIndex(index=json_int(json_field(object_, "index")))
    elif kind == "symbol":
        return StaticKeySymbol(
            symbol=from_json_symbol_key(json_field(object_, "symbol"))
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class SymbolKeyUnique:
    """Unique symbol key from a declaration."""

    unique: destack._generated.dir.symbol.symbol.GlobalSymbolId
    kind: typing.Literal["unique"] = "unique"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_symbol_key(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_symbol_key(self)


@dataclass(frozen=True, slots=True)
class SymbolKeyRegistry:
    """Symbol.for registry key (string is the content of `Symbol.for`)."""

    registry: destack._generated.core.string.StringId
    kind: typing.Literal["registry"] = "registry"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_symbol_key(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_symbol_key(self)


"""Symbol as a key."""
SymbolKey: typing.TypeAlias = SymbolKeyUnique | SymbolKeyRegistry


def encode_symbol_key(writer: BinaryWriter, value: SymbolKey) -> None:
    """Encode one SymbolKey."""
    if value.kind == "unique":
        writer.write_unsigned(0)
        destack._generated.dir.symbol.symbol.encode_global_symbol_id(
            writer, value.unique
        )
    elif value.kind == "registry":
        writer.write_unsigned(1)
        destack._generated.core.string.encode_string_id(writer, value.registry)
    else:
        raise SerdeError("unknown enum variant")


def decode_symbol_key(reader: BinaryReader) -> SymbolKey:
    """Decode one SymbolKey."""
    variant = reader.read_number()

    if variant == 0:
        unique = destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)

        return SymbolKeyUnique(unique=unique)
    elif variant == 1:
        registry = destack._generated.core.string.decode_string_id(reader)

        return SymbolKeyRegistry(registry=registry)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_symbol_key(value: SymbolKey) -> Json:
    """Return one JSON value for one SymbolKey."""
    if value.kind == "unique":
        return {
            "kind": "unique",
            "unique": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
                value.unique
            ),
        }
    elif value.kind == "registry":
        return {
            "kind": "registry",
            "registry": destack._generated.core.string.to_json_string_id(
                value.registry
            ),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_symbol_key(value: Json) -> SymbolKey:
    """Return one SymbolKey from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "unique":
        return SymbolKeyUnique(
            unique=destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
                json_field(object_, "unique")
            )
        )
    elif kind == "registry":
        return SymbolKeyRegistry(
            registry=destack._generated.core.string.from_json_string_id(
                json_field(object_, "registry")
            )
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


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
