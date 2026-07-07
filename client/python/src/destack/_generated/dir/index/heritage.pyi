# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.dir.index.postings
import destack._generated.dir.symbol.symbol
import destack._generated.dir.tree.node
import destack._generated.dir.type.type

@dataclass(frozen=True, slots=True)
class HeritageIndex:
    """Indexed nominal heritage edges."""

    # heritage edges ordered by base symbol
    by_base: Sequence[HeritageEntry]
    # heritage edges ordered by derived symbol
    by_derived: Sequence[HeritageEntry]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> HeritageIndex: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> HeritageIndex: ...

def encode_heritage_index(writer: BinaryWriter, value: HeritageIndex) -> None: ...
def decode_heritage_index(reader: BinaryReader) -> HeritageIndex: ...
def to_json_heritage_index(value: HeritageIndex) -> Json: ...
def from_json_heritage_index(value: Json) -> HeritageIndex: ...

@dataclass(frozen=True, slots=True)
class HeritageEntry:
    """One nominal heritage edge."""

    # the derived nominal symbol
    derived: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the inherited or implemented nominal symbol
    base: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the source heritage node
    source: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the generic arguments used at the heritage site
    arguments: Sequence[destack._generated.dir.type.type.GlobalTypeId]
    # the heritage kind
    kind: HeritageKind

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> HeritageEntry: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> HeritageEntry: ...

def encode_heritage_entry(writer: BinaryWriter, value: HeritageEntry) -> None: ...
def decode_heritage_entry(reader: BinaryReader) -> HeritageEntry: ...
def to_json_heritage_entry(value: HeritageEntry) -> Json: ...
def from_json_heritage_entry(value: Json) -> HeritageEntry: ...

"""Nominal heritage kind."""
HeritageKind: typing.TypeAlias = (
    typing.Literal["extends"] | typing.Literal["implements"]
)

def encode_heritage_kind(writer: BinaryWriter, value: HeritageKind) -> None: ...
def decode_heritage_kind(reader: BinaryReader) -> HeritageKind: ...
def to_json_heritage_kind(value: HeritageKind) -> Json: ...
def from_json_heritage_kind(value: Json) -> HeritageKind: ...

@dataclass(frozen=True, slots=True)
class HeritagePostings:
    """Heritage postings by base symbol."""

    # heritage base postings
    bases: destack._generated.dir.index.postings.Postings
    # heritage derived postings
    derived: destack._generated.dir.index.postings.Postings

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> HeritagePostings: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> HeritagePostings: ...

def encode_heritage_postings(writer: BinaryWriter, value: HeritagePostings) -> None: ...
def decode_heritage_postings(reader: BinaryReader) -> HeritagePostings: ...
def to_json_heritage_postings(value: HeritagePostings) -> Json: ...
def from_json_heritage_postings(value: Json) -> HeritagePostings: ...

__all__ = [
    "HeritageIndex",
    "encode_heritage_index",
    "decode_heritage_index",
    "to_json_heritage_index",
    "from_json_heritage_index",
    "HeritageEntry",
    "encode_heritage_entry",
    "decode_heritage_entry",
    "to_json_heritage_entry",
    "from_json_heritage_entry",
    "HeritageKind",
    "encode_heritage_kind",
    "decode_heritage_kind",
    "to_json_heritage_kind",
    "from_json_heritage_kind",
    "HeritagePostings",
    "encode_heritage_postings",
    "decode_heritage_postings",
    "to_json_heritage_postings",
    "from_json_heritage_postings",
]
