# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.dir.symbol.symbol
import destack._generated.dir.tree.static
import destack._generated.source.file.model.module

@dataclass(frozen=True, slots=True)
class StaticSegment:
    """Static values added by one DIR phase."""

    # the module id of the static segment
    module_id: destack._generated.source.file.model.module.ModuleId
    # the first static id owned by this table segment
    first_static_id: int
    # interned static values
    statics: Sequence[destack._generated.dir.tree.static.StaticTerm]
    # checked static value keyed by symbol
    static_by_symbol_id: Mapping[
        destack._generated.dir.symbol.symbol.GlobalSymbolId,
        destack._generated.dir.tree.static.GlobalStaticId,
    ]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> StaticSegment: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> StaticSegment: ...

def encode_static_segment(writer: BinaryWriter, value: StaticSegment) -> None: ...
def decode_static_segment(reader: BinaryReader) -> StaticSegment: ...
def to_json_static_segment(value: StaticSegment) -> Json: ...
def from_json_static_segment(value: Json) -> StaticSegment: ...

__all__ = [
    "StaticSegment",
    "encode_static_segment",
    "decode_static_segment",
    "to_json_static_segment",
    "from_json_static_segment",
]
