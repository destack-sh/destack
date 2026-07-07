# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.dir.index.postings
import destack._generated.dir.symbol.symbol
import destack._generated.dir.tree.node
import destack._generated.source.file.model.file
import destack._generated.source.file.model.span

@dataclass(frozen=True, slots=True)
class ExportIndex:
    """Indexed exported symbols."""

    # the exports in stable display order
    entries: Sequence[ExportEntry]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ExportIndex: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ExportIndex: ...

def encode_export_index(writer: BinaryWriter, value: ExportIndex) -> None: ...
def decode_export_index(reader: BinaryReader) -> ExportIndex: ...
def to_json_export_index(value: ExportIndex) -> Json: ...
def from_json_export_index(value: Json) -> ExportIndex: ...

@dataclass(frozen=True, slots=True)
class ExportEntry:
    """One indexed exported symbol."""

    # the exported symbol name
    name: str
    # the exported symbol kind
    kind: destack._generated.dir.symbol.symbol.SymbolKind
    # the resolved exported symbol
    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the source node that exposes this export
    source: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the source file
    file: destack._generated.source.file.model.file.FileId
    # the source range
    span: destack._generated.source.file.model.span.Span
    # the module path to use in imports
    module_path: str | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ExportEntry: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ExportEntry: ...

def encode_export_entry(writer: BinaryWriter, value: ExportEntry) -> None: ...
def decode_export_entry(reader: BinaryReader) -> ExportEntry: ...
def to_json_export_entry(value: ExportEntry) -> Json: ...
def from_json_export_entry(value: Json) -> ExportEntry: ...

@dataclass(frozen=True, slots=True)
class ExportPostings:
    """Export postings by name."""

    # export name postings
    names: destack._generated.dir.index.postings.Postings

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ExportPostings: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ExportPostings: ...

def encode_export_postings(writer: BinaryWriter, value: ExportPostings) -> None: ...
def decode_export_postings(reader: BinaryReader) -> ExportPostings: ...
def to_json_export_postings(value: ExportPostings) -> Json: ...
def from_json_export_postings(value: Json) -> ExportPostings: ...

__all__ = [
    "ExportIndex",
    "encode_export_index",
    "decode_export_index",
    "to_json_export_index",
    "from_json_export_index",
    "ExportEntry",
    "encode_export_entry",
    "decode_export_entry",
    "to_json_export_entry",
    "from_json_export_entry",
    "ExportPostings",
    "encode_export_postings",
    "decode_export_postings",
    "to_json_export_postings",
    "from_json_export_postings",
]
