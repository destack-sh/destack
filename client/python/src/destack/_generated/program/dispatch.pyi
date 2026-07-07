# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.core.section

@dataclass(frozen=True, slots=True)
class DispatchTable:
    """Dispatch table carried by one durable program."""

    # virtual tables keyed by dense virtual table id
    virtual_tables: destack._generated.core.section.SectionSlice
    # flattened virtual method function ids
    virtual_methods: destack._generated.core.section.SectionSlice
    # dynamic tables keyed by dense dynamic table id
    dynamic_tables: destack._generated.core.section.SectionSlice
    # flattened dynamic dispatch entries
    dynamic_entries: destack._generated.core.section.SectionSlice
    # dynamic table shapes keyed by constraint type
    dynamic_shapes: destack._generated.core.section.SectionSlice
    # flattened dynamic slots
    dynamic_slots: destack._generated.core.section.SectionSlice

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> DispatchTable: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> DispatchTable: ...

def encode_dispatch_table(writer: BinaryWriter, value: DispatchTable) -> None: ...
def decode_dispatch_table(reader: BinaryReader) -> DispatchTable: ...
def to_json_dispatch_table(value: DispatchTable) -> Json: ...
def from_json_dispatch_table(value: Json) -> DispatchTable: ...

__all__ = [
    "DispatchTable",
    "encode_dispatch_table",
    "decode_dispatch_table",
    "to_json_dispatch_table",
    "from_json_dispatch_table",
]
