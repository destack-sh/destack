# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_field,
    json_object,
)

import destack._generated.program.function


@dataclass(frozen=True, slots=True)
class EntryPoint:
    """One program entrypoint id."""

    value: destack._generated.program.function.FunctionId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_entry_point(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> EntryPoint:
        """Decode one EntryPoint."""
        return decode_entry_point(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_entry_point(self)

    @classmethod
    def from_json(cls, value: Json) -> EntryPoint:
        """Return one EntryPoint from one JSON value."""
        return from_json_entry_point(value)


def encode_entry_point(writer: BinaryWriter, value: EntryPoint) -> None:
    """Encode one EntryPoint."""
    destack._generated.program.function.encode_function_id(writer, value.value)


def decode_entry_point(reader: BinaryReader) -> EntryPoint:
    """Decode one EntryPoint."""
    value_ = destack._generated.program.function.decode_function_id(reader)

    return EntryPoint(
        value=value_,
    )


def to_json_entry_point(value: EntryPoint) -> Json:
    """Return one JSON value for one EntryPoint."""
    return {
        "value": destack._generated.program.function.to_json_function_id(value.value),
    }


def from_json_entry_point(value: Json) -> EntryPoint:
    """Return one EntryPoint from one JSON value."""
    object_ = json_object(value)

    return EntryPoint(
        value=destack._generated.program.function.from_json_function_id(
            json_field(object_, "value")
        ),
    )


__all__ = [
    "EntryPoint",
    "encode_entry_point",
    "decode_entry_point",
    "to_json_entry_point",
    "from_json_entry_point",
]
