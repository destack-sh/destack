# generated client target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.mir.metadata.layout
import destack._generated.mir.tree.node
import destack._generated.mir.tree.type

@dataclass(frozen=True, slots=True)
class TypeTable:
    """Runtime type metadata carried by one durable program."""

    # dense runtime type records keyed by program type id
    types: Sequence[destack._generated.mir.tree.type.Type | None]
    # MIR type id by program type id
    mir_types: Sequence[destack._generated.mir.tree.node.LocalNodeId | None]
    # program type id by MIR type id
    type_ids: Mapping[destack._generated.mir.tree.node.LocalNodeId, TypeId]
    # dense runtime layout ids keyed by program type id
    layout_by_type: Sequence[destack._generated.mir.metadata.layout.LayoutId | None]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TypeTable: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TypeTable: ...

def encode_type_table(writer: BinaryWriter, value: TypeTable) -> None: ...
def decode_type_table(reader: BinaryReader) -> TypeTable: ...
def to_json_type_table(value: TypeTable) -> Json: ...
def from_json_type_table(value: Json) -> TypeTable: ...

"""Durable runtime type id inside one program."""
TypeId: typing.TypeAlias = int

def encode_type_id(writer: BinaryWriter, value: TypeId) -> None: ...
def decode_type_id(reader: BinaryReader) -> TypeId: ...
def to_json_type_id(value: TypeId) -> Json: ...
def from_json_type_id(value: Json) -> TypeId: ...

__all__ = [
    "TypeTable",
    "encode_type_table",
    "decode_type_table",
    "to_json_type_table",
    "from_json_type_table",
    "TypeId",
    "encode_type_id",
    "decode_type_id",
    "to_json_type_id",
    "from_json_type_id",
]
