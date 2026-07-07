# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.core.section

@dataclass(frozen=True, slots=True)
class FunctionTable:
    """Function table carried by one durable program."""

    # dense function entries keyed by program function id
    functions: destack._generated.core.section.SectionSlice
    # flattened function parameter types
    parameters: destack._generated.core.section.SectionSlice
    # exported function names
    exports: destack._generated.core.section.SectionSlice

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
