# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.core.section

@dataclass(frozen=True, slots=True)
class EntryTable:
    """Native entry table keyed by program ids."""

    # native function entries keyed by program function id
    function: destack._generated.core.section.SectionSlice
    # native resume entries keyed by frame state id
    resume: destack._generated.core.section.SectionSlice

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> EntryTable: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> EntryTable: ...

def encode_entry_table(writer: BinaryWriter, value: EntryTable) -> None: ...
def decode_entry_table(reader: BinaryReader) -> EntryTable: ...
def to_json_entry_table(value: EntryTable) -> Json: ...
def from_json_entry_table(value: Json) -> EntryTable: ...

__all__ = [
    "EntryTable",
    "encode_entry_table",
    "decode_entry_table",
    "to_json_entry_table",
    "from_json_entry_table",
]
