# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_field,
    json_object,
)

import destack._generated.core.string


@dataclass(frozen=True, slots=True)
class Symbol:
    """Persistent, mangled identity of a function, global, or type."""

    value: destack._generated.core.string.StringId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_symbol(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Symbol:
        """Decode one Symbol."""
        return decode_symbol(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_symbol(self)

    @classmethod
    def from_json(cls, value: Json) -> Symbol:
        """Return one Symbol from one JSON value."""
        return from_json_symbol(value)


def encode_symbol(writer: BinaryWriter, value: Symbol) -> None:
    """Encode one Symbol."""
    destack._generated.core.string.encode_string_id(writer, value.value)


def decode_symbol(reader: BinaryReader) -> Symbol:
    """Decode one Symbol."""
    value_ = destack._generated.core.string.decode_string_id(reader)

    return Symbol(
        value=value_,
    )


def to_json_symbol(value: Symbol) -> Json:
    """Return one JSON value for one Symbol."""
    return {
        "value": destack._generated.core.string.to_json_string_id(value.value),
    }


def from_json_symbol(value: Json) -> Symbol:
    """Return one Symbol from one JSON value."""
    object_ = json_object(value)

    return Symbol(
        value=destack._generated.core.string.from_json_string_id(
            json_field(object_, "value")
        ),
    )


__all__ = [
    "Symbol",
    "encode_symbol",
    "decode_symbol",
    "to_json_symbol",
    "from_json_symbol",
]
