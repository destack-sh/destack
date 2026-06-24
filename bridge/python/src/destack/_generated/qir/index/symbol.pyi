# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.dir.symbol.symbol
import destack._generated.qir.index.name
import destack._generated.source.file.model.file
import destack._generated.source.file.model.module
import destack._generated.source.file.model.span

@dataclass(frozen=True, slots=True)
class SymbolIndex:
    """Searchable symbol declaration index."""

    # the symbol entries in stable display order
    entries: Sequence[SymbolEntry]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> SymbolIndex: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> SymbolIndex: ...

def encode_symbol_index(writer: BinaryWriter, value: SymbolIndex) -> None: ...
def decode_symbol_index(reader: BinaryReader) -> SymbolIndex: ...
def to_json_symbol_index(value: SymbolIndex) -> Json: ...
def from_json_symbol_index(value: Json) -> SymbolIndex: ...

@dataclass(frozen=True, slots=True)
class SymbolEntry:
    """Searchable workspace symbol entry."""

    # the display name
    name: destack._generated.qir.index.name.Name
    # the symbol kind
    kind: SymbolKind
    # the owning module
    module_id: destack._generated.source.file.model.module.ModuleId
    # the source file
    file_id: destack._generated.source.file.model.file.FileId
    # the source range
    range: destack._generated.source.file.model.span.Span
    # the indexed symbol when known
    symbol_id: destack._generated.dir.symbol.symbol.GlobalSymbolId | None
    # the containing symbol display name
    container_name: str | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> SymbolEntry: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> SymbolEntry: ...

def encode_symbol_entry(writer: BinaryWriter, value: SymbolEntry) -> None: ...
def decode_symbol_entry(reader: BinaryReader) -> SymbolEntry: ...
def to_json_symbol_entry(value: SymbolEntry) -> Json: ...
def from_json_symbol_entry(value: Json) -> SymbolEntry: ...

"""Searchable workspace symbol kind."""
SymbolKind: typing.TypeAlias = (
    typing.Literal["namespace"]
    | typing.Literal["class"]
    | typing.Literal["enum"]
    | typing.Literal["interface"]
    | typing.Literal["function"]
    | typing.Literal["variable"]
    | typing.Literal["constant"]
    | typing.Literal["struct"]
    | typing.Literal["typeParameter"]
)

def encode_symbol_kind(writer: BinaryWriter, value: SymbolKind) -> None: ...
def decode_symbol_kind(reader: BinaryReader) -> SymbolKind: ...
def to_json_symbol_kind(value: SymbolKind) -> Json: ...
def from_json_symbol_kind(value: Json) -> SymbolKind: ...

__all__ = [
    "SymbolIndex",
    "encode_symbol_index",
    "decode_symbol_index",
    "to_json_symbol_index",
    "from_json_symbol_index",
    "SymbolEntry",
    "encode_symbol_entry",
    "decode_symbol_entry",
    "to_json_symbol_entry",
    "from_json_symbol_entry",
    "SymbolKind",
    "encode_symbol_kind",
    "decode_symbol_kind",
    "to_json_symbol_kind",
    "from_json_symbol_kind",
]
