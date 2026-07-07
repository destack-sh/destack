# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.core.section

@dataclass(frozen=True, slots=True)
class ImportTable:
    """Native imports required by one native code payload."""

    # native imports in linker order
    import_: destack._generated.core.section.SectionSlice

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ImportTable: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ImportTable: ...

def encode_import_table(writer: BinaryWriter, value: ImportTable) -> None: ...
def decode_import_table(reader: BinaryReader) -> ImportTable: ...
def to_json_import_table(value: ImportTable) -> Json: ...
def from_json_import_table(value: Json) -> ImportTable: ...

__all__ = [
    "ImportTable",
    "encode_import_table",
    "decode_import_table",
    "to_json_import_table",
    "from_json_import_table",
]
