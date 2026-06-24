# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.dir.symbol.symbol
import destack._generated.source.file.model.module

@dataclass(frozen=True, slots=True)
class ImportIndex:
    """Importable export index."""

    # the import entries in stable display order
    entries: Sequence[ImportEntry]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ImportIndex: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ImportIndex: ...

def encode_import_index(writer: BinaryWriter, value: ImportIndex) -> None: ...
def decode_import_index(reader: BinaryReader) -> ImportIndex: ...
def to_json_import_index(value: ImportIndex) -> Json: ...
def from_json_import_index(value: Json) -> ImportIndex: ...

@dataclass(frozen=True, slots=True)
class ImportEntry:
    """Importable exported symbol entry."""

    # the exported symbol name
    name: str
    # the exported symbol kind
    kind: destack._generated.dir.symbol.symbol.SymbolKind
    # the exported symbol space
    space: destack._generated.dir.symbol.symbol.SymbolSpace
    # the module that exports this symbol
    module_id: destack._generated.source.file.model.module.ModuleId
    # the local exported symbol id
    local_id: destack._generated.dir.symbol.symbol.LocalSymbolId
    # the module path to use in imports
    module_path: str | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ImportEntry: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ImportEntry: ...

def encode_import_entry(writer: BinaryWriter, value: ImportEntry) -> None: ...
def decode_import_entry(reader: BinaryReader) -> ImportEntry: ...
def to_json_import_entry(value: ImportEntry) -> Json: ...
def from_json_import_entry(value: Json) -> ImportEntry: ...

__all__ = [
    "ImportIndex",
    "encode_import_index",
    "decode_import_index",
    "to_json_import_index",
    "from_json_import_index",
    "ImportEntry",
    "encode_import_entry",
    "decode_import_entry",
    "to_json_import_entry",
    "from_json_import_entry",
]
