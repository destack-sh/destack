# generated client target, do not edit

from __future__ import annotations

import builtins

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

@dataclass(frozen=True, slots=True)
class ConstValueAggregate:
    """Constant payload for an aggregate value."""

    aggregate: builtins.bytes | bytearray | Sequence[int]
    kind: typing.Literal["aggregate"] = "aggregate"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Constant value stored in lowered instructions."""
ConstValue: typing.TypeAlias = ConstValueAggregate

def encode_const_value(writer: BinaryWriter, value: ConstValue) -> None: ...
def decode_const_value(reader: BinaryReader) -> ConstValue: ...
def to_json_const_value(value: ConstValue) -> Json: ...
def from_json_const_value(value: Json) -> ConstValue: ...

__all__ = [
    "ConstValue",
    "encode_const_value",
    "decode_const_value",
    "to_json_const_value",
    "from_json_const_value",
    "ConstValueAggregate",
]
