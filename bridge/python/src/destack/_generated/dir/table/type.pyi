# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.dir.symbol.symbol
import destack._generated.dir.tree.node
import destack._generated.dir.type.type
import destack._generated.source.file.model.module

@dataclass(frozen=True, slots=True)
class TypeSegment:
    """Type slots added by one DIR phase."""

    # the module id of the type segment
    module_id: destack._generated.source.file.model.module.ModuleId
    # the first type id owned by this table segment
    first_type_id: int
    # canonical type entries
    types: Sequence[destack._generated.dir.type.type.Type]
    # the source for each type id
    sources: Sequence[destack._generated.dir.tree.node.LocalNodeIdAny]
    # effective checked type keyed by DIR node occurrence
    node_types: Mapping[
        destack._generated.dir.tree.node.GlobalNodeIdAny,
        destack._generated.dir.type.type.GlobalTypeId,
    ]
    # checked declaration type keyed by symbol
    symbol_types: Mapping[
        destack._generated.dir.symbol.symbol.GlobalSymbolId,
        destack._generated.dir.type.type.GlobalTypeId,
    ]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TypeSegment: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TypeSegment: ...

def encode_type_segment(writer: BinaryWriter, value: TypeSegment) -> None: ...
def decode_type_segment(reader: BinaryReader) -> TypeSegment: ...
def to_json_type_segment(value: TypeSegment) -> Json: ...
def from_json_type_segment(value: Json) -> TypeSegment: ...

__all__ = [
    "TypeSegment",
    "encode_type_segment",
    "decode_type_segment",
    "to_json_type_segment",
    "from_json_type_segment",
]
