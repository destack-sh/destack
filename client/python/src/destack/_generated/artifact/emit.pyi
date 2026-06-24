# generated client target, do not edit

from __future__ import annotations

import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

"""Emitted artifact family for a build target."""
EmitFormat: typing.TypeAlias = (
    typing.Literal["js"]
    | typing.Literal["ts"]
    | typing.Literal["wasm"]
    | typing.Literal["native"]
)

def encode_emit_format(writer: BinaryWriter, value: EmitFormat) -> None: ...
def decode_emit_format(reader: BinaryReader) -> EmitFormat: ...
def to_json_emit_format(value: EmitFormat) -> Json: ...
def from_json_emit_format(value: Json) -> EmitFormat: ...

__all__ = [
    "EmitFormat",
    "encode_emit_format",
    "decode_emit_format",
    "to_json_emit_format",
    "from_json_emit_format",
]
