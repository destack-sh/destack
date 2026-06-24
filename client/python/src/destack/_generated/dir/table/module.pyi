# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.dir.symbol.module
import destack._generated.source.file.model.module

@dataclass(frozen=True, slots=True)
class ModuleSegment:
    """Module import edges added by one DIR phase."""

    # the module id of the segment
    module_id: destack._generated.source.file.model.module.ModuleId
    # locally resolved module import edges
    edges: Sequence[destack._generated.dir.symbol.module.ModuleEdge]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ModuleSegment: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ModuleSegment: ...

def encode_module_segment(writer: BinaryWriter, value: ModuleSegment) -> None: ...
def decode_module_segment(reader: BinaryReader) -> ModuleSegment: ...
def to_json_module_segment(value: ModuleSegment) -> Json: ...
def from_json_module_segment(value: Json) -> ModuleSegment: ...

__all__ = [
    "ModuleSegment",
    "encode_module_segment",
    "decode_module_segment",
    "to_json_module_segment",
    "from_json_module_segment",
]
