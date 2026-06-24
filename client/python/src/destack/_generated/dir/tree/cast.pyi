# generated client target, do not edit

from __future__ import annotations

import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

"""How one cast expression entered the DIR."""
CastOrigin: typing.TypeAlias = typing.Literal["explicit"] | typing.Literal["implicit"]

def encode_cast_origin(writer: BinaryWriter, value: CastOrigin) -> None: ...
def decode_cast_origin(reader: BinaryReader) -> CastOrigin: ...
def to_json_cast_origin(value: CastOrigin) -> Json: ...
def from_json_cast_origin(value: Json) -> CastOrigin: ...

__all__ = [
    "CastOrigin",
    "encode_cast_origin",
    "decode_cast_origin",
    "to_json_cast_origin",
    "from_json_cast_origin",
]
