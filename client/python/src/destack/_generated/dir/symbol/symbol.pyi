# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.dir.symbol.key
import destack._generated.dir.symbol.scope
import destack._generated.dir.tree.dependency
import destack._generated.dir.tree.node
import destack._generated.source.file.model.module

@dataclass(frozen=True, slots=True)
class GlobalSymbolId:
    """Global symbol id across modules."""

    # the module id of the global symbol
    module_id: destack._generated.source.file.model.module.ModuleId
    # the local id of the global symbol
    local_id: LocalSymbolId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> GlobalSymbolId: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> GlobalSymbolId: ...

def encode_global_symbol_id(writer: BinaryWriter, value: GlobalSymbolId) -> None: ...
def decode_global_symbol_id(reader: BinaryReader) -> GlobalSymbolId: ...
def to_json_global_symbol_id(value: GlobalSymbolId) -> Json: ...
def from_json_global_symbol_id(value: Json) -> GlobalSymbolId: ...

@dataclass(frozen=True, slots=True)
class LocalSymbolId:
    """Unique identifier for Symbols."""

    # the numeric id
    id: int

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> LocalSymbolId: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> LocalSymbolId: ...

def encode_local_symbol_id(writer: BinaryWriter, value: LocalSymbolId) -> None: ...
def decode_local_symbol_id(reader: BinaryReader) -> LocalSymbolId: ...
def to_json_local_symbol_id(value: LocalSymbolId) -> Json: ...
def from_json_local_symbol_id(value: Json) -> LocalSymbolId: ...

@dataclass(frozen=True, slots=True)
class Symbol:
    """A bindable item or local in a scope."""

    # the scope lookup role of the symbol
    role: SymbolRole
    # the declaration kind of the symbol
    kind: SymbolKind
    # the mutability for value bindings when known
    binding_mutability: destack._generated.dir.tree.node.Mutability | None
    # where this symbol was introduced
    origin: SymbolOrigin
    # the key of the symbol
    key: destack._generated.dir.symbol.key.StaticKey | None
    # the scope that introduces the symbol
    scope: destack._generated.dir.symbol.scope.LocalScope
    # the export kind of the symbol
    export_kind: destack._generated.dir.tree.dependency.ExportKind | None
    # the declaration node that introduced this symbol
    declaration: destack._generated.dir.tree.node.GlobalNodeIdAny | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Symbol: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Symbol: ...

def encode_symbol(writer: BinaryWriter, value: Symbol) -> None: ...
def decode_symbol(reader: BinaryReader) -> Symbol: ...
def to_json_symbol(value: Symbol) -> Json: ...
def from_json_symbol(value: Json) -> Symbol: ...

"""The scope lookup role of a symbol."""
SymbolRole: typing.TypeAlias = (
    typing.Literal["namespace"] | typing.Literal["item"] | typing.Literal["local"]
)

def encode_symbol_role(writer: BinaryWriter, value: SymbolRole) -> None: ...
def decode_symbol_role(reader: BinaryReader) -> SymbolRole: ...
def to_json_symbol_role(value: SymbolRole) -> Json: ...
def from_json_symbol_role(value: Json) -> SymbolRole: ...

"""The declaration kind of a symbol."""
SymbolKind: typing.TypeAlias = (
    typing.Literal["variable"]
    | typing.Literal["import"]
    | typing.Literal["class"]
    | typing.Literal["struct"]
    | typing.Literal["interface"]
    | typing.Literal["newtypeInterface"]
    | typing.Literal["enum"]
    | typing.Literal["enumField"]
    | typing.Literal["function"]
    | typing.Literal["label"]
    | typing.Literal["extension"]
    | typing.Literal["typeAlias"]
    | typing.Literal["genericTypeParameter"]
    | typing.Literal["genericValueParameter"]
    | typing.Literal["associatedType"]
    | typing.Literal["associatedConst"]
    | typing.Literal["newtype"]
)

def encode_symbol_kind(writer: BinaryWriter, value: SymbolKind) -> None: ...
def decode_symbol_kind(reader: BinaryReader) -> SymbolKind: ...
def to_json_symbol_kind(value: SymbolKind) -> Json: ...
def from_json_symbol_kind(value: Json) -> SymbolKind: ...

"""Where a symbol originated in the source."""
SymbolOrigin: typing.TypeAlias = typing.Literal["module"] | typing.Literal["global"]

def encode_symbol_origin(writer: BinaryWriter, value: SymbolOrigin) -> None: ...
def decode_symbol_origin(reader: BinaryReader) -> SymbolOrigin: ...
def to_json_symbol_origin(value: SymbolOrigin) -> Json: ...
def from_json_symbol_origin(value: Json) -> SymbolOrigin: ...

"""The space of a symbol."""
SymbolSpace: typing.TypeAlias = (
    typing.Literal["type"] | typing.Literal["value"] | typing.Literal["label"]
)

def encode_symbol_space(writer: BinaryWriter, value: SymbolSpace) -> None: ...
def decode_symbol_space(reader: BinaryReader) -> SymbolSpace: ...
def to_json_symbol_space(value: SymbolSpace) -> Json: ...
def from_json_symbol_space(value: Json) -> SymbolSpace: ...

__all__ = [
    "GlobalSymbolId",
    "encode_global_symbol_id",
    "decode_global_symbol_id",
    "to_json_global_symbol_id",
    "from_json_global_symbol_id",
    "LocalSymbolId",
    "encode_local_symbol_id",
    "decode_local_symbol_id",
    "to_json_local_symbol_id",
    "from_json_local_symbol_id",
    "Symbol",
    "encode_symbol",
    "decode_symbol",
    "to_json_symbol",
    "from_json_symbol",
    "SymbolRole",
    "encode_symbol_role",
    "decode_symbol_role",
    "to_json_symbol_role",
    "from_json_symbol_role",
    "SymbolKind",
    "encode_symbol_kind",
    "decode_symbol_kind",
    "to_json_symbol_kind",
    "from_json_symbol_kind",
    "SymbolOrigin",
    "encode_symbol_origin",
    "decode_symbol_origin",
    "to_json_symbol_origin",
    "from_json_symbol_origin",
    "SymbolSpace",
    "encode_symbol_space",
    "decode_symbol_space",
    "to_json_symbol_space",
    "from_json_symbol_space",
]
