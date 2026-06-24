# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

from destack._impl.mir.metadata.data import (
    DataLayoutImpl,
)

@dataclass(frozen=True, slots=True)
class DataLayout(DataLayoutImpl):
    """Target data layout for one MIR module."""

    # pointer size in bytes
    pointer_bytes: int

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> DataLayout: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> DataLayout: ...

def encode_data_layout(writer: BinaryWriter, value: DataLayout) -> None: ...
def decode_data_layout(reader: BinaryReader) -> DataLayout: ...
def to_json_data_layout(value: DataLayout) -> Json: ...
def from_json_data_layout(value: Json) -> DataLayout: ...

__all__ = [
    "DataLayout",
    "encode_data_layout",
    "decode_data_layout",
    "to_json_data_layout",
    "from_json_data_layout",
]
