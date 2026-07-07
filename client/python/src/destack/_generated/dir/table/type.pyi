# generated client target, do not edit

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
    # the interned type entries
    types: Sequence[destack._generated.dir.type.type.Type]
    # the structural flags per type, computed at intern time
    flags: Sequence[destack._generated.dir.type.type.TypeFlags]
    # the type id lists referenced by type payloads
    type_ids: ListPool
    # the tuple element lists referenced by type payloads
    elements: ListPool
    # the shape field lists referenced by type payloads
    fields: ListPool
    # the function parameter lists referenced by type payloads
    parameters: ListPool
    # the index signature lists referenced by type payloads
    index_signatures: ListPool
    # the string lists referenced by type payloads
    strings: ListPool
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
    # reduced checked type keyed by surface type
    reduced_types: Mapping[
        destack._generated.dir.type.type.GlobalTypeId,
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

@dataclass(frozen=True, slots=True)
class ListPool:
    """Interned lists of one type payload kind."""

    # the first element owned by this segment
    first: int
    # the stored list elements
    elements: Sequence[destack._generated.dir.type.type.GlobalTypeId]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ListPool: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ListPool: ...

def encode_list_pool(writer: BinaryWriter, value: ListPool) -> None: ...
def decode_list_pool(reader: BinaryReader) -> ListPool: ...
def to_json_list_pool(value: ListPool) -> Json: ...
def from_json_list_pool(value: Json) -> ListPool: ...

__all__ = [
    "TypeSegment",
    "encode_type_segment",
    "decode_type_segment",
    "to_json_type_segment",
    "from_json_type_segment",
    "ListPool",
    "encode_list_pool",
    "decode_list_pool",
    "to_json_list_pool",
    "from_json_list_pool",
]
