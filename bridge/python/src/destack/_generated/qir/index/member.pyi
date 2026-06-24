# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.dir.symbol.symbol
import destack._generated.dir.tree.node
import destack._generated.dir.type.type
import destack._generated.qir.index.name
import destack._generated.source.file.model.file
import destack._generated.source.file.model.module
import destack._generated.source.file.model.span

@dataclass(frozen=True, slots=True)
class MemberIndex:
    """Searchable source member index."""

    # the member entries in stable source order
    entries: Sequence[MemberEntry]
    # member entry indexes ordered by name
    by_name: Sequence[int]
    # member entry indexes ordered by owner symbol
    by_owner: Sequence[int]
    # member entry indexes ordered by declaring symbol
    by_declaring: Sequence[int]
    # member entry indexes ordered by member symbol
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
    """Searchable source member entry."""

    # the member name
    name: destack._generated.qir.index.name.Name
    # the member kind
    kind: MemberKind
    # the owner symbol
    owner_symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the symbol whose declaration contains this member
    declaring_symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the member symbol
    member_symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the source node that declares the member
    source_node: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the module containing the member declaration
    module_id: destack._generated.source.file.model.module.ModuleId
    # the source file
    file_id: destack._generated.source.file.model.file.FileId
    # the source range
    range: destack._generated.source.file.model.span.Span
    # the containing symbol display name
    container_name: str | None
    # the checked type of the member when known
    declared_type: destack._generated.dir.type.type.GlobalTypeId | None
    # the member source family
    source: MemberSource
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

"""Searchable source member kind."""
MemberKind: typing.TypeAlias = (
    typing.Literal["field"]
    | typing.Literal["method"]
    | typing.Literal["associatedType"]
    | typing.Literal["associatedConst"]
    | typing.Literal["variant"]
)

def encode_member_kind(writer: BinaryWriter, value: MemberKind) -> None: ...
def decode_member_kind(reader: BinaryReader) -> MemberKind: ...
def to_json_member_kind(value: MemberKind) -> Json: ...
def from_json_member_kind(value: Json) -> MemberKind: ...

"""Searchable source member source family."""
MemberSource: typing.TypeAlias = (
    typing.Literal["declaration"]
    | typing.Literal["typeMember"]
    | typing.Literal["enumVariant"]
    | typing.Literal["extension"]
)

def encode_member_source(writer: BinaryWriter, value: MemberSource) -> None: ...
def decode_member_source(reader: BinaryReader) -> MemberSource: ...
def to_json_member_source(value: MemberSource) -> Json: ...
def from_json_member_source(value: Json) -> MemberSource: ...

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
    "MemberSource",
    "encode_member_source",
    "decode_member_source",
    "to_json_member_source",
    "from_json_member_source",
]
