# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

from destack._impl.mir.tree.value import (
    ValueSliceImpl,
)

"""SSA value (virtual register)."""
Value: typing.TypeAlias = int

def encode_value(writer: BinaryWriter, value: Value) -> None: ...
def decode_value(reader: BinaryReader) -> Value: ...
def to_json_value(value: Value) -> Json: ...
def from_json_value(value: Json) -> Value: ...

@dataclass(frozen=True, slots=True)
class ValueSlice(ValueSliceImpl):
    """Compact reference to a value list stored in the MIR tree."""

    # start index in the value buffer
    start: int
    # number of values in the slice
    count: int

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ValueSlice: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ValueSlice: ...

def encode_value_slice(writer: BinaryWriter, value: ValueSlice) -> None: ...
def decode_value_slice(reader: BinaryReader) -> ValueSlice: ...
def to_json_value_slice(value: ValueSlice) -> Json: ...
def from_json_value_slice(value: Json) -> ValueSlice: ...

__all__ = [
    "Value",
    "encode_value",
    "decode_value",
    "to_json_value",
    "from_json_value",
    "ValueSlice",
    "encode_value_slice",
    "decode_value_slice",
    "to_json_value_slice",
    "from_json_value_slice",
]
