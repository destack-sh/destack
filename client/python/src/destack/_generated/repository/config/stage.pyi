# generated client target, do not edit

from __future__ import annotations

import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

"""Package release stage."""
Stage: typing.TypeAlias = (
    typing.Literal["experimental"]
    | typing.Literal["alpha"]
    | typing.Literal["beta"]
    | typing.Literal["stable"]
)

def encode_stage(writer: BinaryWriter, value: Stage) -> None: ...
def decode_stage(reader: BinaryReader) -> Stage: ...
def to_json_stage(value: Stage) -> Json: ...
def from_json_stage(value: Json) -> Stage: ...

__all__ = [
    "Stage",
    "encode_stage",
    "decode_stage",
    "to_json_stage",
    "from_json_stage",
]
