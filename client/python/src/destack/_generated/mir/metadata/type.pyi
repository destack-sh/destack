# generated client target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

from destack._impl.mir.metadata.type import (
    TypeMetadataImpl,
)

import destack._generated.core.string
import destack._generated.mir.tree.node

@dataclass(frozen=True, slots=True)
class TypeMetadata(TypeMetadataImpl):
    """Canonical type metadata for one MIR module."""

    # nominal lineage keyed by type id
    lineage_by_type: Mapping[destack._generated.mir.tree.node.LocalNodeId, TypeLineage]
    # runtime type descriptor globals keyed by type id
    descriptor_by_type: Mapping[
        destack._generated.mir.tree.node.LocalNodeId,
        destack._generated.mir.tree.node.LocalNodeId,
    ]
    # canonical display names keyed by type id
    display_name_by_type: Mapping[
        destack._generated.mir.tree.node.LocalNodeId,
        destack._generated.core.string.StringId,
    ]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TypeMetadata: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TypeMetadata: ...

def encode_type_metadata(writer: BinaryWriter, value: TypeMetadata) -> None: ...
def decode_type_metadata(reader: BinaryReader) -> TypeMetadata: ...
def to_json_type_metadata(value: TypeMetadata) -> Json: ...
def from_json_type_metadata(value: Json) -> TypeMetadata: ...

@dataclass(frozen=True, slots=True)
class TypeLineage:
    """Lineage metadata for nominal types."""

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
    "TypeMetadata",
    "encode_type_metadata",
    "decode_type_metadata",
    "to_json_type_metadata",
    "from_json_type_metadata",
    "TypeLineage",
    "encode_type_lineage",
    "decode_type_lineage",
    "to_json_type_lineage",
    "from_json_type_lineage",
]
