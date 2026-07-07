# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.core.section

@dataclass(frozen=True, slots=True)
class GlobalTable:
    """Global table carried by one program."""

    # global records keyed by dense global id
    globals: destack._generated.core.section.SectionSlice

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> GlobalTable: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> GlobalTable: ...

def encode_global_table(writer: BinaryWriter, value: GlobalTable) -> None: ...
def decode_global_table(reader: BinaryReader) -> GlobalTable: ...
def to_json_global_table(value: GlobalTable) -> Json: ...
def from_json_global_table(value: Json) -> GlobalTable: ...

__all__ = [
    "GlobalTable",
    "encode_global_table",
    "decode_global_table",
    "to_json_global_table",
    "from_json_global_table",
]
