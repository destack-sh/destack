# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.dir.symbol.symbol
import destack._generated.source.file.model.module

@dataclass(frozen=True, slots=True)
class ReferenceIndex:
    """Reference target membership index."""

    # the reference entries ordered by target symbol
    by_target: Sequence[ReferenceEntry]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ReferenceIndex: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ReferenceIndex: ...

def encode_reference_index(writer: BinaryWriter, value: ReferenceIndex) -> None: ...
def decode_reference_index(reader: BinaryReader) -> ReferenceIndex: ...
def to_json_reference_index(value: ReferenceIndex) -> Json: ...
def from_json_reference_index(value: Json) -> ReferenceIndex: ...

@dataclass(frozen=True, slots=True)
class ReferenceEntry:
    """One module's membership in one reference target set."""

    # the referenced symbol
    target_symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the module that may reference the symbol
    module_id: destack._generated.source.file.model.module.ModuleId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ReferenceEntry: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ReferenceEntry: ...

def encode_reference_entry(writer: BinaryWriter, value: ReferenceEntry) -> None: ...
def decode_reference_entry(reader: BinaryReader) -> ReferenceEntry: ...
def to_json_reference_entry(value: ReferenceEntry) -> Json: ...
def from_json_reference_entry(value: Json) -> ReferenceEntry: ...

__all__ = [
    "ReferenceIndex",
    "encode_reference_index",
    "decode_reference_index",
    "to_json_reference_index",
    "from_json_reference_index",
    "ReferenceEntry",
    "encode_reference_entry",
    "decode_reference_entry",
    "to_json_reference_entry",
    "from_json_reference_entry",
]
