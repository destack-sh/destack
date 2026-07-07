# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.core.section

@dataclass(frozen=True, slots=True)
class TraceTable:
    """Compact heap trace table stored in program sections."""

    # top-level trace roots indexed by TraceId
    roots: destack._generated.core.section.SectionSlice
    # flat trace entries
    entries: destack._generated.core.section.SectionSlice
    # flattened trace byte offsets
    offsets: destack._generated.core.section.SectionSlice
    # flattened child entry ids
    children: destack._generated.core.section.SectionSlice
    # flattened tagged variant entries
    variants: destack._generated.core.section.SectionSlice

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TraceTable: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TraceTable: ...

def encode_trace_table(writer: BinaryWriter, value: TraceTable) -> None: ...
def decode_trace_table(reader: BinaryReader) -> TraceTable: ...
def to_json_trace_table(value: TraceTable) -> Json: ...
def from_json_trace_table(value: Json) -> TraceTable: ...

__all__ = [
    "TraceTable",
    "encode_trace_table",
    "decode_trace_table",
    "to_json_trace_table",
    "from_json_trace_table",
]
