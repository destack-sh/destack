# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.core.section

@dataclass(frozen=True, slots=True)
class FrameTable:
    """Physical execution frame layout and materialization tables."""

    # frame materializations by frame state id
    materializations: destack._generated.core.section.SectionSlice
    # frame layouts by id
    layouts: destack._generated.core.section.SectionSlice
    # flattened frame slots
    slots: destack._generated.core.section.SectionSlice
    # flattened copied materialization slots
    copied_slots: destack._generated.core.section.SectionSlice

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> FrameTable: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> FrameTable: ...

def encode_frame_table(writer: BinaryWriter, value: FrameTable) -> None: ...
def decode_frame_table(reader: BinaryReader) -> FrameTable: ...
def to_json_frame_table(value: FrameTable) -> Json: ...
def from_json_frame_table(value: Json) -> FrameTable: ...

__all__ = [
    "FrameTable",
    "encode_frame_table",
    "decode_frame_table",
    "to_json_frame_table",
    "from_json_frame_table",
]
