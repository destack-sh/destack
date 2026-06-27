# generated client target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.core.string
import destack._generated.mir.tree.node

@dataclass(frozen=True, slots=True)
class TypeTable:
    """Canonical type table for one MIR module."""

    # nominal lineage keyed by type id
    lineage_by_type: Mapping[destack._generated.mir.tree.node.LocalNodeId, TypeLineage]
    # canonical display names keyed by type id
    display_name_by_type: Mapping[
        destack._generated.mir.tree.node.LocalNodeId,
        destack._generated.core.string.StringId,
    ]

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

@dataclass(frozen=True, slots=True)
class TypeLineage:
    """Lineage tables for nominal types."""

    # optional parent type for class inheritance
    parent: destack._generated.mir.tree.node.LocalNodeId | None
    # interfaces implemented by this type
    interfaces: Sequence[destack._generated.mir.tree.node.LocalNodeId]
    # true when the type is sealed to external extension
    is_sealed: bool
    # true when the type is final and cannot be subclassed
    is_final: bool
    # true when the type is abstract and cannot be instantiated
    is_abstract: bool
    # true when the type represents an interface
    is_interface: bool

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TypeLineage: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TypeLineage: ...

def encode_type_lineage(writer: BinaryWriter, value: TypeLineage) -> None: ...
def decode_type_lineage(reader: BinaryReader) -> TypeLineage: ...
def to_json_type_lineage(value: TypeLineage) -> Json: ...
def from_json_type_lineage(value: Json) -> TypeLineage: ...

__all__ = [
    "TypeTable",
    "encode_type_table",
    "decode_type_table",
    "to_json_type_table",
    "from_json_type_table",
    "TypeLineage",
    "encode_type_lineage",
    "decode_type_lineage",
    "to_json_type_lineage",
    "from_json_type_lineage",
]
