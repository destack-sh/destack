# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.core.section

@dataclass(frozen=True, slots=True)
class LayoutTable:
    """Shared layout table for runtime values."""

    # layout entries indexed by LayoutId
    layouts: destack._generated.core.section.SectionSlice
    # flattened field layout entries
    fields: destack._generated.core.section.SectionSlice
    # flattened variant case layout entries
    variants: destack._generated.core.section.SectionSlice
    # flattened signature parameter type entries
    parameters: destack._generated.core.section.SectionSlice
    # flattened tensor sharding axis entries
    tensor_axes: destack._generated.core.section.SectionSlice

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> LayoutTable: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> LayoutTable: ...

def encode_layout_table(writer: BinaryWriter, value: LayoutTable) -> None: ...
def decode_layout_table(reader: BinaryReader) -> LayoutTable: ...
def to_json_layout_table(value: LayoutTable) -> Json: ...
def from_json_layout_table(value: Json) -> LayoutTable: ...

__all__ = [
    "LayoutTable",
    "encode_layout_table",
    "decode_layout_table",
    "to_json_layout_table",
    "from_json_layout_table",
]
