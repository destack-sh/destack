# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.dir.index.postings
import destack._generated.dir.symbol.symbol
import destack._generated.dir.table.definition
import destack._generated.dir.tree.node
import destack._generated.dir.type.type
import destack._generated.source.file.model.file
import destack._generated.source.file.model.span

@dataclass(frozen=True, slots=True)
class MemberIndex:
    """Indexed checked members."""

    # the members in stable source order
    entries: Sequence[MemberEntry]
    # member indexes ordered by owner symbol
    by_owner: Sequence[int]
    # member indexes ordered by declaring symbol
    by_declaring: Sequence[int]
    # member indexes ordered by member symbol
    by_symbol: Sequence[int]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> MemberIndex: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> MemberIndex: ...

def encode_member_index(writer: BinaryWriter, value: MemberIndex) -> None: ...
def decode_member_index(reader: BinaryReader) -> MemberIndex: ...
def to_json_member_index(value: MemberIndex) -> Json: ...
def from_json_member_index(value: Json) -> MemberIndex: ...

@dataclass(frozen=True, slots=True)
class MemberEntry:
    """One indexed member."""

    # the member name
    name: str
    # the member kind
    kind: MemberKind
    # the symbol whose member surface receives this member
    owner: destack._generated.dir.symbol.symbol.GlobalSymbolId | None
    # the symbol whose definition declares this member
    declaring: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the member symbol when this member declares one
    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId | None
    # the source node that defines the member
    source: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the source file
    file: destack._generated.source.file.model.file.FileId
    # the source range
    span: destack._generated.source.file.model.span.Span
    # the containing symbol display name
    container: str | None
    # the checked type of the member when known
    ty: destack._generated.dir.type.type.GlobalTypeId | None
    # the member origin
    origin: MemberOrigin
    # the checked member space
    space: destack._generated.dir.table.definition.MemberSpace
    # whether implementers must supply this member
    is_abstract: bool
    # whether this member overrides an inherited member
    is_override: bool
    # whether this member supplies a default implementation
    is_default: bool
    # whether this member belongs to the static surface
    is_static: bool

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> MemberEntry: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> MemberEntry: ...

def encode_member_entry(writer: BinaryWriter, value: MemberEntry) -> None: ...
def decode_member_entry(reader: BinaryReader) -> MemberEntry: ...
def to_json_member_entry(value: MemberEntry) -> Json: ...
def from_json_member_entry(value: Json) -> MemberEntry: ...

"""Indexed member kind."""
MemberKind: typing.TypeAlias = (
    typing.Literal["field"]
    | typing.Literal["method"]
    | typing.Literal["constructor"]
    | typing.Literal["callSignature"]
    | typing.Literal["constructSignature"]
    | typing.Literal["indexSignature"]
    | typing.Literal["associatedType"]
    | typing.Literal["associatedConst"]
    | typing.Literal["variant"]
)

def encode_member_kind(writer: BinaryWriter, value: MemberKind) -> None: ...
def decode_member_kind(reader: BinaryReader) -> MemberKind: ...
def to_json_member_kind(value: MemberKind) -> Json: ...
def from_json_member_kind(value: Json) -> MemberKind: ...

"""Indexed member origin."""
MemberOrigin: typing.TypeAlias = (
    typing.Literal["definition"] | typing.Literal["extension"]
)

def encode_member_origin(writer: BinaryWriter, value: MemberOrigin) -> None: ...
def decode_member_origin(reader: BinaryReader) -> MemberOrigin: ...
def to_json_member_origin(value: MemberOrigin) -> Json: ...
def from_json_member_origin(value: Json) -> MemberOrigin: ...

@dataclass(frozen=True, slots=True)
class MemberPostings:
    """Member postings by lookup key."""

    # member name postings
    names: destack._generated.dir.index.postings.Postings
    # member owner postings
    owners: destack._generated.dir.index.postings.Postings
    # member declaring symbol postings
    declaring: destack._generated.dir.index.postings.Postings

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> MemberPostings: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> MemberPostings: ...

def encode_member_postings(writer: BinaryWriter, value: MemberPostings) -> None: ...
def decode_member_postings(reader: BinaryReader) -> MemberPostings: ...
def to_json_member_postings(value: MemberPostings) -> Json: ...
def from_json_member_postings(value: Json) -> MemberPostings: ...

__all__ = [
    "MemberIndex",
    "encode_member_index",
    "decode_member_index",
    "to_json_member_index",
    "from_json_member_index",
    "MemberEntry",
    "encode_member_entry",
    "decode_member_entry",
    "to_json_member_entry",
    "from_json_member_entry",
    "MemberKind",
    "encode_member_kind",
    "decode_member_kind",
    "to_json_member_kind",
    "from_json_member_kind",
    "MemberOrigin",
    "encode_member_origin",
    "decode_member_origin",
    "to_json_member_origin",
    "from_json_member_origin",
    "MemberPostings",
    "encode_member_postings",
    "decode_member_postings",
    "to_json_member_postings",
    "from_json_member_postings",
]
