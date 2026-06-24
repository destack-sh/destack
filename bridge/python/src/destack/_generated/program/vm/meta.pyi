# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

from destack._impl.program.vm.meta import (
    ReferenceMetaImpl,
)

@dataclass(frozen=True, slots=True)
class ReferenceMeta(ReferenceMetaImpl):
    """Metadata for reference values."""

    bits: int

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ReferenceMeta: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ReferenceMeta: ...

def encode_reference_meta(writer: BinaryWriter, value: ReferenceMeta) -> None: ...
def decode_reference_meta(reader: BinaryReader) -> ReferenceMeta: ...
def to_json_reference_meta(value: ReferenceMeta) -> Json: ...
def from_json_reference_meta(value: Json) -> ReferenceMeta: ...

__all__ = [
    "ReferenceMeta",
    "encode_reference_meta",
    "decode_reference_meta",
    "to_json_reference_meta",
    "from_json_reference_meta",
]
