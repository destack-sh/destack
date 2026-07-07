# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.dir.index.postings
import destack._generated.dir.tree.node
import destack._generated.source.file.model.file
import destack._generated.source.file.model.module
import destack._generated.source.file.model.span

@dataclass(frozen=True, slots=True)
class SpecifierIndex:
    """Module specifier rewrite index."""

    # the resolved specifiers ordered by target path
    by_target_path: Sequence[tuple[str, SpecifierEntry]]
    # the specifiers without a resolved target path
    unresolved: Sequence[SpecifierEntry]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> SpecifierIndex: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> SpecifierIndex: ...

def encode_specifier_index(writer: BinaryWriter, value: SpecifierIndex) -> None: ...
def decode_specifier_index(reader: BinaryReader) -> SpecifierIndex: ...
def to_json_specifier_index(value: SpecifierIndex) -> Json: ...
def from_json_specifier_index(value: Json) -> SpecifierIndex: ...

@dataclass(frozen=True, slots=True)
class SpecifierEntry:
    """One indexed module specifier."""

    # the source file
    file: destack._generated.source.file.model.file.FileId
    # the source node that owns the specifier
    source: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the source range
    span: destack._generated.source.file.model.span.Span
    # the module specifier kind
    kind: SpecifierKind
    # the specifier text
    text: str
    # the semantic target module when resolved
    target_module: destack._generated.source.file.model.module.ModuleId | None
    # the semantic target path when known
    target_path: str | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> SpecifierEntry: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> SpecifierEntry: ...

def encode_specifier_entry(writer: BinaryWriter, value: SpecifierEntry) -> None: ...
def decode_specifier_entry(reader: BinaryReader) -> SpecifierEntry: ...
def to_json_specifier_entry(value: SpecifierEntry) -> Json: ...
def from_json_specifier_entry(value: Json) -> SpecifierEntry: ...

"""Kind of module specifier."""
SpecifierKind: typing.TypeAlias = typing.Literal["import"] | typing.Literal["export"]

def encode_specifier_kind(writer: BinaryWriter, value: SpecifierKind) -> None: ...
def decode_specifier_kind(reader: BinaryReader) -> SpecifierKind: ...
def to_json_specifier_kind(value: SpecifierKind) -> Json: ...
def from_json_specifier_kind(value: Json) -> SpecifierKind: ...

@dataclass(frozen=True, slots=True)
class SpecifierPostings:
    """Module specifier postings by resolved target path."""

    # resolved target path postings
    paths: destack._generated.dir.index.postings.Postings
    # modules that contain unresolved specifiers
    unresolved: Sequence[int]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> SpecifierPostings: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> SpecifierPostings: ...

def encode_specifier_postings(
    writer: BinaryWriter, value: SpecifierPostings
) -> None: ...
def decode_specifier_postings(reader: BinaryReader) -> SpecifierPostings: ...
def to_json_specifier_postings(value: SpecifierPostings) -> Json: ...
def from_json_specifier_postings(value: Json) -> SpecifierPostings: ...

__all__ = [
    "SpecifierIndex",
    "encode_specifier_index",
    "decode_specifier_index",
    "to_json_specifier_index",
    "from_json_specifier_index",
    "SpecifierEntry",
    "encode_specifier_entry",
    "decode_specifier_entry",
    "to_json_specifier_entry",
    "from_json_specifier_entry",
    "SpecifierKind",
    "encode_specifier_kind",
    "decode_specifier_kind",
    "to_json_specifier_kind",
    "from_json_specifier_kind",
    "SpecifierPostings",
    "encode_specifier_postings",
    "decode_specifier_postings",
    "to_json_specifier_postings",
    "from_json_specifier_postings",
]
