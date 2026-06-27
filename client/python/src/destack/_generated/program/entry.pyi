# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.program.function

@dataclass(frozen=True, slots=True)
class EntryPoint:
    """One program entrypoint id."""

    value: destack._generated.program.function.FunctionId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> EntryPoint: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> EntryPoint: ...

def encode_entry_point(writer: BinaryWriter, value: EntryPoint) -> None: ...
def decode_entry_point(reader: BinaryReader) -> EntryPoint: ...
def to_json_entry_point(value: EntryPoint) -> Json: ...
def from_json_entry_point(value: Json) -> EntryPoint: ...

__all__ = [
    "EntryPoint",
    "encode_entry_point",
    "decode_entry_point",
    "to_json_entry_point",
    "from_json_entry_point",
]
