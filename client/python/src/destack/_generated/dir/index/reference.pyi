# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.dir.index.postings
import destack._generated.dir.symbol.symbol
import destack._generated.dir.tree.node
import destack._generated.source.file.model.span

@dataclass(frozen=True, slots=True)
class ReferenceIndex:
    """Reference occurrence index."""

    # the references ordered by target symbol
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
    """One indexed reference occurrence."""

    # the referenced symbol
    target: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the source node that references the symbol
    source: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the source range
    span: destack._generated.source.file.model.span.Span
    # the reference kind
    kind: ReferenceKind

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

"""Kind of indexed reference occurrence."""
ReferenceKind: typing.TypeAlias = (
    typing.Literal["name"]
    | typing.Literal["member"]
    | typing.Literal["call"]
    | typing.Literal["construct"]
    | typing.Literal["dependency"]
    | typing.Literal["type"]
    | typing.Literal["storage"]
)

def encode_reference_kind(writer: BinaryWriter, value: ReferenceKind) -> None: ...
def decode_reference_kind(reader: BinaryReader) -> ReferenceKind: ...
def to_json_reference_kind(value: ReferenceKind) -> Json: ...
def from_json_reference_kind(value: Json) -> ReferenceKind: ...

@dataclass(frozen=True, slots=True)
class ReferencePostings:
    """Reference postings by target symbol."""

    # reference target postings
    targets: destack._generated.dir.index.postings.Postings

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ReferencePostings: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ReferencePostings: ...

def encode_reference_postings(
    writer: BinaryWriter, value: ReferencePostings
) -> None: ...
def decode_reference_postings(reader: BinaryReader) -> ReferencePostings: ...
def to_json_reference_postings(value: ReferencePostings) -> Json: ...
def from_json_reference_postings(value: Json) -> ReferencePostings: ...

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
    "ReferenceKind",
    "encode_reference_kind",
    "decode_reference_kind",
    "to_json_reference_kind",
    "from_json_reference_kind",
    "ReferencePostings",
    "encode_reference_postings",
    "decode_reference_postings",
    "to_json_reference_postings",
    "from_json_reference_postings",
]
