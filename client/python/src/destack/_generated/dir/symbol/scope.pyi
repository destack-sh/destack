# generated client target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.dir.symbol.key
import destack._generated.dir.symbol.symbol
import destack._generated.source.file.model.module

@dataclass(frozen=True, slots=True)
class LocalScope:
    """Local scope id and mark pair used for node and symbol insertion."""

    # the scope id
    id: LocalScopeId
    # the visible binding mark
    mark: LocalScopeMark

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> LocalScope: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> LocalScope: ...

def encode_local_scope(writer: BinaryWriter, value: LocalScope) -> None: ...
def decode_local_scope(reader: BinaryReader) -> LocalScope: ...
def to_json_local_scope(value: LocalScope) -> Json: ...
def from_json_local_scope(value: Json) -> LocalScope: ...

"""Unique identifier for local scopes."""
LocalScopeId: typing.TypeAlias = int

def encode_local_scope_id(writer: BinaryWriter, value: LocalScopeId) -> None: ...
def decode_local_scope_id(reader: BinaryReader) -> LocalScopeId: ...
def to_json_local_scope_id(value: LocalScopeId) -> Json: ...
def from_json_local_scope_id(value: Json) -> LocalScopeId: ...

"""Mark a position in a scope."""
LocalScopeMark: typing.TypeAlias = int

def encode_local_scope_mark(writer: BinaryWriter, value: LocalScopeMark) -> None: ...
def decode_local_scope_mark(reader: BinaryReader) -> LocalScopeMark: ...
def to_json_local_scope_mark(value: LocalScopeMark) -> Json: ...
def from_json_local_scope_mark(value: Json) -> LocalScopeMark: ...

@dataclass(frozen=True, slots=True)
class Scope:
    """A lexical container for symbols."""

    # the kind of the scope
    kind: ScopeKind
    # the parent scope
    parent: LocalScope | None
    # the owner of the scope
    owner: destack._generated.dir.symbol.symbol.LocalSymbolId | None
    # the bindings in lexical order
    bindings: Sequence[ScopeBinding]
    # index for named bindings
    index: ScopeIndex
    # the children scopes
    children: Sequence[LocalScopeId]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Scope: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Scope: ...

def encode_scope(writer: BinaryWriter, value: Scope) -> None: ...
def decode_scope(reader: BinaryReader) -> Scope: ...
def to_json_scope(value: Scope) -> Json: ...
def from_json_scope(value: Json) -> Scope: ...

"""The kind of a scope."""
ScopeKind: typing.TypeAlias = (
    typing.Literal["module"]
    | typing.Literal["global"]
    | typing.Literal["namespace"]
    | typing.Literal["function"]
    | typing.Literal["type"]
    | typing.Literal["typeConditional"]
    | typing.Literal["label"]
    | typing.Literal["block"]
)

def encode_scope_kind(writer: BinaryWriter, value: ScopeKind) -> None: ...
def decode_scope_kind(reader: BinaryReader) -> ScopeKind: ...
def to_json_scope_kind(value: ScopeKind) -> Json: ...
def from_json_scope_kind(value: Json) -> ScopeKind: ...

@dataclass(frozen=True, slots=True)
class ScopeBinding:
    """One binding entry in lexical order."""

    # the binding key
    key: destack._generated.dir.symbol.key.StaticKey | None
    # the bound symbol
    symbol: destack._generated.dir.symbol.symbol.LocalSymbolId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ScopeBinding: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ScopeBinding: ...

def encode_scope_binding(writer: BinaryWriter, value: ScopeBinding) -> None: ...
def decode_scope_binding(reader: BinaryReader) -> ScopeBinding: ...
def to_json_scope_binding(value: ScopeBinding) -> Json: ...
def from_json_scope_binding(value: Json) -> ScopeBinding: ...

@dataclass(frozen=True, slots=True)
class ScopeIndexEmpty:
    """No named bindings."""

    kind: typing.Literal["empty"] = "empty"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ScopeIndexSmall:
    """A small inline table for common tiny scopes."""

    # binding indices grouped by key
    entries: Sequence[ScopeIndexEntry]
    kind: typing.Literal["small"] = "small"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ScopeIndexLarge:
    """A keyed table for larger scopes."""

    # binding indices grouped by key
    table: Mapping[destack._generated.dir.symbol.key.StaticKey, Sequence[int]]
    kind: typing.Literal["large"] = "large"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""A compact name index for one lexical scope."""
ScopeIndex: typing.TypeAlias = ScopeIndexEmpty | ScopeIndexSmall | ScopeIndexLarge

def encode_scope_index(writer: BinaryWriter, value: ScopeIndex) -> None: ...
def decode_scope_index(reader: BinaryReader) -> ScopeIndex: ...
def to_json_scope_index(value: ScopeIndex) -> Json: ...
def from_json_scope_index(value: Json) -> ScopeIndex: ...

@dataclass(frozen=True, slots=True)
class ScopeIndexEntry:
    """Binding indices for one key in a small scope lookup."""

    # the binding key
    key: destack._generated.dir.symbol.key.StaticKey
    # binding indices in lexical order
    indices: Sequence[int]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ScopeIndexEntry: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ScopeIndexEntry: ...

def encode_scope_index_entry(writer: BinaryWriter, value: ScopeIndexEntry) -> None: ...
def decode_scope_index_entry(reader: BinaryReader) -> ScopeIndexEntry: ...
def to_json_scope_index_entry(value: ScopeIndexEntry) -> Json: ...
def from_json_scope_index_entry(value: Json) -> ScopeIndexEntry: ...

@dataclass(frozen=True, slots=True)
class GlobalScopeId:
    """Global scope id across modules."""

    # the module id of the global scope
    module_id: destack._generated.source.file.model.module.ModuleId
    # the local id of the global scope
    local_id: LocalScopeId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> GlobalScopeId: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> GlobalScopeId: ...

def encode_global_scope_id(writer: BinaryWriter, value: GlobalScopeId) -> None: ...
def decode_global_scope_id(reader: BinaryReader) -> GlobalScopeId: ...
def to_json_global_scope_id(value: GlobalScopeId) -> Json: ...
def from_json_global_scope_id(value: Json) -> GlobalScopeId: ...

__all__ = [
    "LocalScope",
    "encode_local_scope",
    "decode_local_scope",
    "to_json_local_scope",
    "from_json_local_scope",
    "LocalScopeId",
    "encode_local_scope_id",
    "decode_local_scope_id",
    "to_json_local_scope_id",
    "from_json_local_scope_id",
    "LocalScopeMark",
    "encode_local_scope_mark",
    "decode_local_scope_mark",
    "to_json_local_scope_mark",
    "from_json_local_scope_mark",
    "Scope",
    "encode_scope",
    "decode_scope",
    "to_json_scope",
    "from_json_scope",
    "ScopeKind",
    "encode_scope_kind",
    "decode_scope_kind",
    "to_json_scope_kind",
    "from_json_scope_kind",
    "ScopeBinding",
    "encode_scope_binding",
    "decode_scope_binding",
    "to_json_scope_binding",
    "from_json_scope_binding",
    "ScopeIndex",
    "encode_scope_index",
    "decode_scope_index",
    "to_json_scope_index",
    "from_json_scope_index",
    "ScopeIndexEmpty",
    "ScopeIndexSmall",
    "ScopeIndexLarge",
    "ScopeIndexEntry",
    "encode_scope_index_entry",
    "decode_scope_index_entry",
    "to_json_scope_index_entry",
    "from_json_scope_index_entry",
    "GlobalScopeId",
    "encode_global_scope_id",
    "decode_global_scope_id",
    "to_json_global_scope_id",
    "from_json_global_scope_id",
]
