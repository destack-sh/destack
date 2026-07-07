# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.dir.index.postings
import destack._generated.dir.symbol.symbol
import destack._generated.dir.tree.node
import destack._generated.dir.type.type
import destack._generated.source.file.model.file
import destack._generated.source.file.model.span

@dataclass(frozen=True, slots=True)
class SymbolIndex:
    """Indexed declared symbols."""

    # the symbols in stable display order
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
    """One indexed declared symbol."""

    # the display name
    name: str
    # the checked symbol kind
    kind: destack._generated.dir.symbol.symbol.SymbolKind
    # the checked symbol role
    role: destack._generated.dir.symbol.symbol.SymbolRole
    # the symbol id
    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the source node that declares the symbol
    source: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the source file
    file: destack._generated.source.file.model.file.FileId
    # the source range
    span: destack._generated.source.file.model.span.Span
    # the checked type of the symbol when known
    ty: destack._generated.dir.type.type.GlobalTypeId | None
    # the containing declaration display name
    container: str | None
    # the binding mutability when this is a value binding
    mutability: destack._generated.dir.tree.node.Mutability | None
    # whether this symbol is exported from its declaring module
    is_exported: bool

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

@dataclass(frozen=True, slots=True)
class SymbolPostings:
    """Symbol postings by name."""

    # symbol name postings
    names: destack._generated.dir.index.postings.Postings

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> SymbolPostings: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> SymbolPostings: ...

def encode_symbol_postings(writer: BinaryWriter, value: SymbolPostings) -> None: ...
def decode_symbol_postings(reader: BinaryReader) -> SymbolPostings: ...
def to_json_symbol_postings(value: SymbolPostings) -> Json: ...
def from_json_symbol_postings(value: Json) -> SymbolPostings: ...

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
    "SymbolPostings",
    "encode_symbol_postings",
    "decode_symbol_postings",
    "to_json_symbol_postings",
    "from_json_symbol_postings",
]
