# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.core.section

@dataclass(frozen=True, slots=True)
class FunctionTable:
    """Lowered VM function registry owned by one program."""

    # lowered functions by dense local index
    functions: destack._generated.core.section.SectionSlice
    # flattened instruction entries
    code: destack._generated.core.section.SectionSlice
    # flattened block entries
    blocks: destack._generated.core.section.SectionSlice
    # flattened argument move slots
    argument_pool: destack._generated.core.section.SectionSlice
    # flattened move pairs
    move_pool: destack._generated.core.section.SectionSlice
    # dense call target entries keyed by program function id
    call_targets: destack._generated.core.section.SectionSlice

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> FunctionTable: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> FunctionTable: ...

def encode_function_table(writer: BinaryWriter, value: FunctionTable) -> None: ...
def decode_function_table(reader: BinaryReader) -> FunctionTable: ...
def to_json_function_table(value: FunctionTable) -> Json: ...
def from_json_function_table(value: Json) -> FunctionTable: ...

__all__ = [
    "FunctionTable",
    "encode_function_table",
    "decode_function_table",
    "to_json_function_table",
    "from_json_function_table",
]
