# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.dir.index.postings
import destack._generated.dir.symbol.symbol
import destack._generated.dir.table.definition
import destack._generated.dir.tree.node
import destack._generated.dir.type.type
import destack._generated.source.file.model.file
import destack._generated.source.file.model.span

@dataclass(frozen=True, slots=True)
class ExtensionIndex:
    """Indexed checked extensions."""

    # extensions ordered by root symbol
    by_root: Sequence[ExtensionEntry]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ExtensionIndex: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ExtensionIndex: ...

def encode_extension_index(writer: BinaryWriter, value: ExtensionIndex) -> None: ...
def decode_extension_index(reader: BinaryReader) -> ExtensionIndex: ...
def to_json_extension_index(value: ExtensionIndex) -> Json: ...
def from_json_extension_index(value: Json) -> ExtensionIndex: ...

@dataclass(frozen=True, slots=True)
class ExtensionEntry:
    """One indexed extension."""

    # the extension declaration symbol
    declaration: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the source extension declaration node
    source: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the source file
    file: destack._generated.source.file.model.file.FileId
    # the source range
    span: destack._generated.source.file.model.span.Span
    # the canonical lookup root when this is a rooted extension
    root: destack._generated.dir.symbol.symbol.GlobalSymbolId | None
    # the checked receiver type
    ty: destack._generated.dir.type.type.GlobalTypeId
    # the extension form
    form: destack._generated.dir.table.definition.ExtensionForm

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ExtensionEntry: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ExtensionEntry: ...

def encode_extension_entry(writer: BinaryWriter, value: ExtensionEntry) -> None: ...
def decode_extension_entry(reader: BinaryReader) -> ExtensionEntry: ...
def to_json_extension_entry(value: ExtensionEntry) -> Json: ...
def from_json_extension_entry(value: Json) -> ExtensionEntry: ...

@dataclass(frozen=True, slots=True)
class ExtensionPostings:
    """Extension postings by root symbol."""

    # rooted extension postings
    roots: destack._generated.dir.index.postings.Postings
    # modules that contain blanket extensions
    blankets: Sequence[int]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ExtensionPostings: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ExtensionPostings: ...

def encode_extension_postings(
    writer: BinaryWriter, value: ExtensionPostings
) -> None: ...
def decode_extension_postings(reader: BinaryReader) -> ExtensionPostings: ...
def to_json_extension_postings(value: ExtensionPostings) -> Json: ...
def from_json_extension_postings(value: Json) -> ExtensionPostings: ...

__all__ = [
    "ExtensionIndex",
    "encode_extension_index",
    "decode_extension_index",
    "to_json_extension_index",
    "from_json_extension_index",
    "ExtensionEntry",
    "encode_extension_entry",
    "decode_extension_entry",
    "to_json_extension_entry",
    "from_json_extension_entry",
    "ExtensionPostings",
    "encode_extension_postings",
    "decode_extension_postings",
    "to_json_extension_postings",
    "from_json_extension_postings",
]
