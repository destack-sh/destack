# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.core.section

@dataclass(frozen=True, slots=True)
class TypeTable:
    """Runtime type table carried by one durable program."""

    # dense runtime type descriptors keyed by program type id
    descriptors: destack._generated.core.section.SectionSlice
    # flattened runtime supertype ids
    supertypes: destack._generated.core.section.SectionSlice

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

__all__ = [
    "TypeTable",
    "encode_type_table",
    "decode_type_table",
    "to_json_type_table",
    "from_json_type_table",
]
