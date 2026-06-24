# generated client target, do not edit

from __future__ import annotations

import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_string,
)

"""Emitted artifact family for a build target."""
EmitFormat: typing.TypeAlias = (
    typing.Literal["js"]
    | typing.Literal["ts"]
    | typing.Literal["wasm"]
    | typing.Literal["native"]
)


def encode_emit_format(writer: BinaryWriter, value: EmitFormat) -> None:
    """Encode one EmitFormat."""
    if value == "js":
        writer.write_unsigned(0)
    elif value == "ts":
        writer.write_unsigned(1)
    elif value == "wasm":
        writer.write_unsigned(2)
    elif value == "native":
        writer.write_unsigned(3)
    else:
        raise SerdeError("unknown enum variant")


def decode_emit_format(reader: BinaryReader) -> EmitFormat:
    """Decode one EmitFormat."""
    variant = reader.read_number()

    if variant == 0:
        return "js"
    elif variant == 1:
        return "ts"
    elif variant == 2:
        return "wasm"
    elif variant == 3:
        return "native"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_emit_format(value: EmitFormat) -> Json:
    """Return one JSON value for one EmitFormat."""
    return value


def from_json_emit_format(value: Json) -> EmitFormat:
    """Return one EmitFormat from one JSON value."""
    variant = json_string(value)

    if variant == "js":
        return "js"
    elif variant == "ts":
        return "ts"
    elif variant == "wasm":
        return "wasm"
    elif variant == "native":
        return "native"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


__all__ = [
    "EmitFormat",
    "encode_emit_format",
    "decode_emit_format",
    "to_json_emit_format",
    "from_json_emit_format",
]
