# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.core.section

@dataclass(frozen=True, slots=True)
class StringTable:
    """Section-backed program string table."""

    # string entries sorted by stable string id
    entries: destack._generated.core.section.SectionSlice
    # concatenated UTF-8 string bytes
    bytes: destack._generated.core.section.SectionSlice

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> StringTable: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> StringTable: ...

def encode_string_table(writer: BinaryWriter, value: StringTable) -> None: ...
def decode_string_table(reader: BinaryReader) -> StringTable: ...
def to_json_string_table(value: StringTable) -> Json: ...
def from_json_string_table(value: Json) -> StringTable: ...

__all__ = [
    "StringTable",
    "encode_string_table",
    "decode_string_table",
    "to_json_string_table",
    "from_json_string_table",
]
